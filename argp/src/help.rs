// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2023 Jakub Jirutka <jakub@jirutka.cz>
// SPDX-FileCopyrightText: 2020 Google LLC

//! Struct in this module are all used by the generated code. They can be used
//! outside the library, but without any guarantees - there may be breaking
//! changes between minor versions.

#![allow(missing_docs)]

use std::fmt;
use std::iter;
use std::ops::Deref;
use std::ptr;

const INDENT: &str = "  ";
const DESC_MIN_INDENT: usize = 8;
const DESC_MAX_INDENT: usize = 30;
const SECTION_SEPARATOR: &str = "\n\n";
const WRAP_WIDTH: usize = 80;

const HELP_OPT: OptionArgInfo = OptionArgInfo {
    usage: "",
    names: "-h, --help",
    description: "Show this help message and exit.",
    global: true,
};

/// Help message generator.
#[derive(Debug)]
pub struct Help {
    info: &'static HelpInfo,
    command_name: String,
    global_options: Vec<&'static OptionArgInfo>,
}

/// Information about a specific (sub)command used for generating a help message.
#[derive(Debug)]
pub struct HelpInfo {
    pub description: &'static str,
    pub positionals: &'static [OptionArgInfo],
    pub options: &'static [OptionArgInfo],
    pub commands: Option<CommandsHelpInfo>,
    pub footer: &'static str,
}

/// A nested struct in [HelpInfo] used for generating the Commands section in
/// a help message.
#[derive(Debug)]
pub struct CommandsHelpInfo {
    pub usage: &'static str,
    pub subcommands: &'static [&'static CommandInfo],
    pub dynamic_subcommands: fn() -> &'static [&'static CommandInfo],
}

/// Information about a particular command used for generating a help message.
/// Unlike the other structures in this module, this one is considered stable.
#[derive(Debug)]
pub struct CommandInfo {
    /// The name of the command.
    pub name: &'static str,
    /// A short description of the command's functionality.
    pub description: &'static str,
}

/// Information about a specific option or positional argument used for
/// generating a help message.
#[derive(Debug)]
pub struct OptionArgInfo {
    pub usage: &'static str,
    pub names: &'static str,
    pub description: &'static str,
    /// Whether to propagate this option down to subcommands. This is valid only
    /// for options and flags, not for positional arguments.
    pub global: bool,
}

impl Help {
    /// Generates a help message.
    pub fn generate(&self) -> String {
        let info = self.info;

        let options = self
            .global_options
            .iter()
            .map(Deref::deref)
            .chain(info.options)
            .chain(iter::once(&HELP_OPT));
        let options_and_args = options.clone().chain(info.positionals);

        let mut out = String::from("Usage: ");
        out.push_str(&self.command_name);

        for usage in options_and_args
            .clone()
            .map(|r| r.usage)
            .filter(|s| !s.is_empty())
        {
            out.push(' ');
            out.push_str(usage);
        }

        if let Some(cmds) = &info.commands {
            out.push(' ');
            out.push_str(cmds.usage);
        }

        out.push_str(SECTION_SEPARATOR);
        out.push_str(
            &info
                .description
                .replace("{command_name}", &self.command_name),
        );

        let subcommands = if let Some(cmds) = &info.commands {
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
            options_and_args
                .map(|r| r.names)
                .chain(subcommands.iter().map(|r| r.name)),
        );

        write_opts_section(&mut out, "Arguments:", info.positionals.iter(), desc_indent);
        write_opts_section(&mut out, "Options:", options, desc_indent);
        write_cmds_section(&mut out, "Commands:", &subcommands, desc_indent);

        if !info.footer.is_empty() {
            out.push_str(SECTION_SEPARATOR);
            out.push_str(&info.footer.replace("{command_name}", &self.command_name));
        }

        out.push('\n');

        out
    }
}

impl fmt::Display for Help {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.generate().fmt(f)
    }
}

impl PartialEq for Help {
    fn eq(&self, other: &Self) -> bool {
        if !ptr::eq(self.info, other.info) {
            return false;
        }
        if self.command_name != other.command_name {
            return false;
        }
        if self.global_options.len() != other.global_options.len() {
            return false;
        }
        self.global_options
            .iter()
            .zip(other.global_options.iter())
            .all(|(a, b)| ptr::eq(*a, *b))
    }
}

impl HelpInfo {
    /// Creates a new `Help` generator instance.
    ///
    /// - `command_name`: The identifier for the current command.
    /// - `global_options`: Information about additional global options (from
    ///   ancestors) to add to the generated help message.
    #[inline]
    pub const fn help(
        &'static self,
        command_name: String,
        global_options: Vec<&'static OptionArgInfo>,
    ) -> Help {
        Help {
            info: self,
            command_name,
            global_options,
        }
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
    opts: impl Iterator<Item = &'a OptionArgInfo>,
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
        line.extend(iter::repeat(' ').take(num_spaces));
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
