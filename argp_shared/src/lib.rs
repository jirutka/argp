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
const DESCRIPTION_INDENT: usize = 20;
const SECTION_SEPARATOR: &str = "\n\n";
const WRAP_WIDTH: usize = 80;

impl<'a> Help<'a> {
    pub fn generate(&self, command_name: String) -> String {
        let mut out = String::from("Usage: ");
        out.push_str(&command_name);
        out.push_str(self.usage);

        out.push_str(SECTION_SEPARATOR);
        out.push_str(&self.description.replace("{command_name}", &command_name));

        write_section(&mut out, "Positional Arguments:", self.positionals);

        let mut options = self.options.to_vec();
        options.push(("-h, --help", "Show this help message and exit"));
        write_section(&mut out, "Options:", &options);

        write_section(&mut out, "Commands:", self.subcommands);

        if !self.footer.is_empty() {
            out.push_str(SECTION_SEPARATOR);
            out.push_str(&self.footer.replace("{command_name}", &command_name));
        }

        out.push('\n');

        out
    }
}

fn write_section(out: &mut String, title: &str, items: &[StrPair]) {
    if !items.is_empty() {
        out.push_str(SECTION_SEPARATOR);
        out.push_str(title);
        for item in items {
            write_description(out, *item);
        }
    }
}

fn write_description(out: &mut String, item: StrPair) {
    let mut current_line = INDENT.to_string();
    current_line.push_str(item.0);

    if item.1.is_empty() {
        new_line(&mut current_line, out);
        return;
    }

    if !indent_description(&mut current_line) {
        // Start the description on a new line if the flag names already
        // add up to more than DESCRIPTION_INDENT.
        new_line(&mut current_line, out);
    }

    let mut words = item.1.split(' ').peekable();
    while let Some(first_word) = words.next() {
        indent_description(&mut current_line);
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

// Indent the current line in to DESCRIPTION_INDENT chars.
// Returns a boolean indicating whether or not spacing was added.
fn indent_description(line: &mut String) -> bool {
    let cur_len = char_len(line);
    if cur_len < DESCRIPTION_INDENT {
        let num_spaces = DESCRIPTION_INDENT - cur_len;
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
