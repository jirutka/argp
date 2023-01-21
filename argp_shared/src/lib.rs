// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2023 Jakub Jirutka <jakub@jirutka.cz>
// SPDX-FileCopyrightText: 2020 Google LLC

//! Shared functionality between argp_derive and the argp runtime.
//!
//! This library is intended only for internal use by these two crates.

/// Information about a particular command used for output.
pub struct CommandInfo<'a> {
    /// The name of the command.
    pub name: &'a str,
    /// A short description of the command's functionality.
    pub description: &'a str,
}

type StrPair<'a> = (&'a str, &'a str);

pub struct Help<'a> {
    pub usage: &'a str,
    pub description: &'a str,
    pub positionals: &'a [StrPair<'a>],
    pub options: &'a [StrPair<'a>],
    pub subcommands: &'a [StrPair<'a>],
    pub footer: &'a str,
}

const INDENT: &str = "  ";
const DESC_MIN_INDENT: usize = 8;
const DESC_MAX_INDENT: usize = 30;
const SECTION_SEPARATOR: &str = "\n\n";
const WRAP_WIDTH: usize = 80;

impl<'a> Help<'a> {
    pub fn generate(&self, command_name: String) -> String {
        let mut out = String::from("Usage: ");
        out.push_str(&command_name);
        out.push_str(self.usage);

        out.push_str(SECTION_SEPARATOR);
        out.push_str(&self.description.replace("{command_name}", &command_name));

        let mut options = self.options.to_vec();
        options.push(("-h, --help", "Show this help message and exit"));

        // Computes the indentation width of the description (right) column based
        // on width of the names/flags in the left column.
        let desc_indent = compute_desc_indent(
            self.positionals.iter().chain(&options).chain(self.subcommands).map(|t| t.0),
        );

        write_section(&mut out, "Positional Arguments:", self.positionals, desc_indent);
        write_section(&mut out, "Options:", &options, desc_indent);

        write_section(&mut out, "Commands:", self.subcommands, desc_indent);

        if !self.footer.is_empty() {
            out.push_str(SECTION_SEPARATOR);
            out.push_str(&self.footer.replace("{command_name}", &command_name));
        }

        out.push('\n');

        out
    }
}

fn compute_desc_indent<'a>(names: impl Iterator<Item = &'a str>) -> usize {
    names
        .map(|name| INDENT.len() + char_len(name) + 2)
        .filter(|width| *width <= DESC_MAX_INDENT)
        .max()
        .unwrap_or(0)
        .max(DESC_MIN_INDENT)
}

fn write_section(out: &mut String, title: &str, items: &[StrPair], desc_indent: usize) {
    if !items.is_empty() {
        out.push_str(SECTION_SEPARATOR);
        out.push_str(title);
        for item in items {
            write_description(out, *item, desc_indent);
        }
    }
}

fn write_description(out: &mut String, item: StrPair, indent_width: usize) {
    let mut current_line = INDENT.to_string();
    current_line.push_str(item.0);

    if item.1.is_empty() {
        new_line(&mut current_line, out);
        return;
    }

    if !indent_description(&mut current_line, indent_width) {
        // Start the description on a new line if the flag names already
        // add up to more than `indent`.
        new_line(&mut current_line, out);
    }

    let mut words = item.1.split(' ').peekable();
    while let Some(first_word) = words.next() {
        indent_description(&mut current_line, indent_width);
        current_line.push_str(first_word);

        'inner: while let Some(&word) = words.peek() {
            if (char_len(&current_line) + char_len(word) + 1) > WRAP_WIDTH {
                new_line(&mut current_line, out);
                break 'inner;
            } else {
                // advance the iterator
                let _ = words.next();
                current_line.push(' ');
                current_line.push_str(word);
            }
        }
    }
    new_line(&mut current_line, out);
}

// Indent the current line in to the `width` chars.
// Returns a boolean indicating whether or not spacing was added.
fn indent_description(line: &mut String, width: usize) -> bool {
    let cur_len = char_len(line);
    if cur_len < width {
        let num_spaces = width - cur_len;
        line.extend(std::iter::repeat(' ').take(num_spaces));
        true
    } else {
        false
    }
}

fn char_len(s: &str) -> usize {
    s.chars().count()
}

// Append a newline and the current line to the output,
// clearing the current line.
fn new_line(current_line: &mut String, out: &mut String) {
    out.push('\n');
    out.push_str(current_line);
    current_line.truncate(0);
}
