// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2023 Jakub Jirutka <jakub@jirutka.cz>
// SPDX-FileCopyrightText: 2020 Google LLC

//! Struct in this module are all used by the generated code. They can be used
//! outside the library, but without any guarantees - there may be breaking
//! changes between minor versions.

#![allow(missing_docs)]

use std::fmt;
use std::iter;
use std::ops::{Deref, Range};
use std::ptr;

use crate::term_size;

const INDENT: &str = "  ";
const DESC_MIN_INDENT: usize = 8;
const DESC_MAX_INDENT: usize = 30;
const SECTION_SEPARATOR: &str = "\n\n";

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
    /// The usage words to be printed in the **Usage** pattern
    /// (`<command> [<args>]` or `[<command>] [<args>]`, literally).
    pub usage: &'static str,
    /// A list of subcommands info.
    pub subcommands: &'static [&'static CommandInfo],
    /// A function that returns a list of subcommands discovered at runtime.
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
    /// The usage word to be printed in the **Usage** pattern (e.g. `[--foo]`,
    /// `[-f <arg>]`, `[<arg>...]`, `<arg>`, ...). This string is generated in
    /// `argp_derive::help`.
    pub usage: &'static str,

    /// The usage string that will printed in the left column of the **Options**
    /// or **Arguments** section. If this is an option, it contains the short
    /// option (if defined), the long option and the argument name if it has one
    /// (e.g. `-f, --foo`, `-f, --foo <arg>`, `'    --foo'`). If this is a
    /// positional argument, it contains the argument name (e.g. `arg`). This
    /// string is generated in `argp_derive::help`.
    pub names: &'static str,

    /// The description of the option/argument to be printed in the right
    /// column of the **Options** or **Arguments** section.
    pub description: &'static str,

    /// Whether to propagate this option down to subcommands. This is valid only
    /// for options and switches, not for positional arguments.
    pub global: bool,
}

/// Style preferences for the Help message generator.
///
/// **Important**: This struct may be extended with more fields in the future,
/// so always initialise it using [`HelpStyle::default()`] (or
/// [`Default::default()`]), for example:
///
/// ```
/// use argp::help::HelpStyle;
///
/// HelpStyle {
///     blank_lines_spacing: 1,
///     ..HelpStyle::default()
/// };
/// ```
#[derive(Debug)]
pub struct HelpStyle {
    /// Specifies the number of blank lines that will be inserted between
    /// descriptions of commands and options. Default is `0`.
    pub blank_lines_spacing: usize,

    /// Specifies the minimum and maximum number of characters to wrap the help
    /// output. If the terminal size is not available (see [`term_size`]), the
    /// output is wrapped to the lower bound of this range.
    /// Default is `80..120`.
    pub wrap_width_range: Range<usize>,
}

impl HelpStyle {
    /// Returns the default help style. Unlike the [`Default`] implementation,
    /// this function is `const`.
    pub const fn default() -> Self {
        Self {
            blank_lines_spacing: 0,
            wrap_width_range: 80..120,
        }
    }

    fn wrap_width(&self) -> usize {
        let Range { start, end } = self.wrap_width_range;

        if start == end {
            start
        } else {
            term_size::term_cols()
                .map(|cols| cols.clamp(start, end))
                .unwrap_or(start)
        }
    }
}

impl Default for HelpStyle {
    fn default() -> Self {
        HelpStyle::default()
    }
}

impl Help {
    /// Generates a help message using the default style.
    pub fn generate_default(&self) -> String {
        self.generate(&HelpStyle::default())
    }

    /// Generates a help message.
    pub fn generate(&self, style: &HelpStyle) -> String {
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

        let mut sw = SectionsWriter {
            out: &mut out,
            desc_indent,
            style,
            wrap_width: style.wrap_width(),
        };
        sw.write_opts_section("Arguments:", info.positionals.iter());
        sw.write_opts_section("Options:", options);
        sw.write_cmds_section("Commands:", &subcommands);

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
        self.generate_default().fmt(f)
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

struct SectionsWriter<'a> {
    desc_indent: usize,
    out: &'a mut String,
    style: &'a HelpStyle,
    wrap_width: usize,
}

impl<'a> SectionsWriter<'_> {
    fn write_opts_section(&mut self, title: &str, opts: impl Iterator<Item = &'a OptionArgInfo>) {
        // NOTE: greedy positional has empty names and description, to be excluded
        // from the Positional Arguments section.
        for (i, opt) in opts.filter(|r| !r.names.is_empty()).enumerate() {
            if i == 0 {
                self.out.push_str(SECTION_SEPARATOR);
                self.out.push_str(title);
            } else {
                self.append_blank_lines();
            }
            self.write_description(opt.names, opt.description);
        }
    }

    fn write_cmds_section(&mut self, title: &str, cmds: &[&CommandInfo]) {
        if !cmds.is_empty() {
            self.out.push_str(SECTION_SEPARATOR);
            self.out.push_str(title);

            for (i, cmd) in cmds.iter().enumerate() {
                if i != 0 {
                    self.append_blank_lines();
                }
                self.write_description(cmd.name, cmd.description);
            }
        }
    }

    fn write_description(&mut self, names: &str, desc: &str) {
        let mut current_line = INDENT.to_string();
        current_line.push_str(names);

        if desc.is_empty() {
            self.new_line(&mut current_line);
            return;
        }

        if !self.indent_description(&mut current_line) {
            // Start the description on a new line if the flag names already
            // add up to more than `indent`.
            self.new_line(&mut current_line);
        }

        let mut words = desc.split(' ').peekable();
        while let Some(first_word) = words.next() {
            self.indent_description(&mut current_line);
            current_line.push_str(first_word);

            'inner: while let Some(&word) = words.peek() {
                if (char_len(&current_line) + char_len(word) + 1) > self.wrap_width {
                    self.new_line(&mut current_line);
                    break 'inner;
                } else {
                    // advance the iterator
                    let _ = words.next();
                    current_line.push(' ');
                    current_line.push_str(word);
                }
            }
        }
        self.new_line(&mut current_line);
    }

    /// Indents the current line in to the `width` chars.
    /// Returns a boolean indicating whether or not spacing was added.
    fn indent_description(&self, line: &mut String) -> bool {
        let cur_len = char_len(line);

        if cur_len < self.desc_indent {
            let num_spaces = self.desc_indent - cur_len;
            line.extend(iter::repeat(' ').take(num_spaces));
            true
        } else {
            false
        }
    }

    /// Appends a newline and the current line to the output,
    /// clearing the current line.
    fn new_line(&mut self, current_line: &mut String) {
        self.out.push('\n');
        self.out.push_str(current_line);
        current_line.truncate(0);
    }

    fn append_blank_lines(&mut self) {
        let count = self.style.blank_lines_spacing;
        if count > 0 {
            self.out.extend(iter::repeat('\n').take(count))
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

fn char_len(s: &str) -> usize {
    s.chars().count()
}
