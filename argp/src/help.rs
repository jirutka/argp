// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2023 Jakub Jirutka <jakub@jirutka.cz>
// SPDX-FileCopyrightText: 2020 Google LLC

#![allow(missing_docs)]

use std::iter;
use std::ops::Deref;

/// Information about a particular command used for generating a help message.
pub struct CommandInfo<'a> {
    /// The name of the command.
    pub name: &'a str,
    /// A short description of the command's functionality.
    pub description: &'a str,
}

/// Information about a specific option or positional argument used for
/// generating a help message.
pub struct OptionArgInfo<'a> {
    pub usage: &'a str,
    pub names: &'a str,
    pub description: &'a str,
}

/// Information about a specific (sub)command used for generating a help message.
pub struct Help<'a> {
    pub description: &'a str,
    pub positionals: &'a [OptionArgInfo<'a>],
    pub options: &'a [OptionArgInfo<'a>],
    pub commands: Option<HelpCommands<'a>>,
    pub footer: &'a str,
}

/// A nested struct in [Help] used for generating the Commands section in
/// a help message.
pub struct HelpCommands<'a> {
    pub usage: &'a str,
    pub subcommands: &'a [&'a CommandInfo<'a>],
    pub dynamic_subcommands: fn() -> &'a [&'a CommandInfo<'a>],
}

const INDENT: &str = "  ";
const DESC_MIN_INDENT: usize = 8;
const DESC_MAX_INDENT: usize = 30;
const SECTION_SEPARATOR: &str = "\n\n";
const WRAP_WIDTH: usize = 80;

const HELP_OPT: OptionArgInfo = OptionArgInfo {
    usage: "",
    names: "-h, --help",
    description: "Show this help message and exit",
};

impl<'a> Help<'a> {
    pub fn generate(&self, command_name: &[&str]) -> String {
        let command_name = command_name.join(" ");

        let mut out = String::from("Usage: ");
        out.push_str(&command_name);

        let usages = self.options.iter().chain(self.positionals).map(|r| r.usage);
        for usage in usages.filter(|s| !s.is_empty()) {
            out.push(' ');
            out.push_str(usage);
        }

        if let Some(cmds) = &self.commands {
            out.push(' ');
            out.push_str(cmds.usage);
        }

        out.push_str(SECTION_SEPARATOR);
        out.push_str(&self.description.replace("{command_name}", &command_name));

        let options = self.options.iter().chain(iter::once(&HELP_OPT));
        let subcommands = if let Some(cmds) = &self.commands {
            cmds.subcommands
                .iter()
                .chain((cmds.dynamic_subcommands)().iter())
                .map(Deref::deref)
                .collect()
        } else {
            Vec::new()
        };

        // Computes the indentation width of the description (right) column based
        // on width of the names/flags in the left column.
        let desc_indent = compute_desc_indent(
            self.positionals
                .iter()
                .chain(options.clone())
                .map(|r| r.names)
                .chain(subcommands.iter().map(|r| r.name)),
        );

        write_opts_section(&mut out, "Positional Arguments:", self.positionals.iter(), desc_indent);
        write_opts_section(&mut out, "Options:", options, desc_indent);
        write_cmds_section(&mut out, "Commands:", &subcommands, desc_indent);

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

fn write_opts_section<'a>(
    out: &mut String,
    title: &str,
    opts: impl Iterator<Item = &'a OptionArgInfo<'a>>,
    desc_indent: usize,
) {
    // NOTE: greedy positional has empty names and description, to be excluded
    // from the Positional Arguments section.
    for (i, opt) in opts.filter(|r| !r.names.is_empty()).enumerate() {
        if i == 0 {
            out.push_str(SECTION_SEPARATOR);
            out.push_str(title);
        }
        write_description(out, opt.names, opt.description, desc_indent);
    }
}

fn write_cmds_section(out: &mut String, title: &str, cmds: &[&CommandInfo], desc_indent: usize) {
    if !cmds.is_empty() {
        out.push_str(SECTION_SEPARATOR);
        out.push_str(title);
        for cmd in cmds {
            write_description(out, cmd.name, cmd.description, desc_indent);
        }
    }
}

fn write_description(out: &mut String, names: &str, desc: &str, indent_width: usize) {
    let mut current_line = INDENT.to_string();
    current_line.push_str(names);

    if desc.is_empty() {
        new_line(&mut current_line, out);
        return;
    }

    if !indent_description(&mut current_line, indent_width) {
        // Start the description on a new line if the flag names already
        // add up to more than `indent`.
        new_line(&mut current_line, out);
    }

    let mut words = desc.split(' ').peekable();
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
