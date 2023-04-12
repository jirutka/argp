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
const SECTION_SEPARATOR: &str = "\n";

const HELP_OPT: OptionArgInfo = OptionArgInfo {
    usage: "",
    description: ("-h, --help", "Show this help message and exit."),
    global: true,
};

/// Help message generator.
#[derive(Debug)]
pub struct Help {
    info: &'static HelpInfo,
    command_name: String,
    global_options: Vec<&'static OptionArgInfo>,
}

/// Information about a specific (sub)command used for generating a help
/// message.
#[derive(Debug)]
pub struct HelpInfo {
    pub description: &'static str,
    pub positionals: &'static [OptionArgInfo],
    pub options: &'static [OptionArgInfo],
    pub commands: Option<CommandsHelpInfo>,
    pub footer: &'static str,
}

/// A nested struct in [`HelpInfo`] used for generating the Commands section in
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

    /// The option/argument description to be printed in the **Options** and
    /// **Arguments** section, respectively, in two columns. If this is an
    /// option, then the left string contains the short option (if defined), the
    /// long option and the argument name if it has one (e.g. `-f, --foo`, `-f,
    /// --foo <arg>`, `'    --foo'`). If this is a positional argument, it
    /// contains the argument name (e.g. `arg`). This value is generated in
    /// `argp_derive::help`.
    pub description: (&'static str, &'static str),

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

    /// Specifies the indentation of the descriptions of arguments, options and
    /// commands (the right column). Positive numbers are interpreted as a
    /// *fixed* number of characters from the left, negative numbers as the
    /// *maximum* indentation of the descriptions as a percentage of the wrap
    /// width.
    ///
    /// If the argument/option flags/command (the left column) is wider than the
    /// (calculated) indentation, its description starts on the next line.
    ///
    /// Default is `-40` (i.e. max 40 % of the wrap width)
    pub description_indent: i8,

    /// If `false` (default), a flag (short _or_ long) for each option will be
    /// included in the **Usage** (e.g. `Usage: myprog [--verbose] [-h]`). If
    /// `true`, options will only be represented by string `[options]`.
    pub short_usage: bool,

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
            description_indent: -40,
            short_usage: false,
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

        let options = self.options();
        let options_and_args = options.clone().chain(info.positionals);
        let subcommands = self.subcommands();

        let wrap_width = style.wrap_width();

        let description_indent = if style.description_indent < 0 {
            let percent = style.description_indent.unsigned_abs() as f32;
            let max_indent = (wrap_width as f32 / 100.0 * percent) as usize;

            let left_column = options_and_args
                .clone()
                .map(|r| r.description.0)
                .chain(subcommands.iter().map(|r| r.name));

            // Calculates the maximum width of the content of the left column
            // that is below the max_indent threshold.
            left_column
                .map(|s| INDENT.len() + chars_count(s) + 2)
                .filter(|width| *width <= max_indent)
                .max()
                .unwrap_or(8)
                .min(wrap_width)
        } else {
            (style.description_indent as usize).min(wrap_width)
        };

        let mut w = HelpWriter {
            blank_lines_spacing: &"\n".repeat(style.blank_lines_spacing),
            buf: String::new(),
            command_name: &self.command_name,
            description_indent,
            wrap_width,
        };

        let subcommands_usage = info.commands.iter().flat_map(|r| r.usage.split(' '));

        if style.short_usage {
            w.write_usage(
                "Usage:",
                iter::once("[options]")
                    .chain(info.positionals.iter().map(|r| r.usage))
                    .chain(subcommands_usage),
            );
        } else {
            w.write_usage("Usage:", options_and_args.map(|r| r.usage).chain(subcommands_usage));
        }

        w.write_paragraphs(info.description);

        if !info.positionals.is_empty() {
            w.write_section("Arguments:", info.positionals.iter().map(|r| r.description));
        }
        w.write_section("Options:", options.map(|r| r.description));

        if !subcommands.is_empty() {
            w.write_section("Commands:", subcommands.iter().map(|r| (r.name, r.description)));
        }
        if !info.footer.is_empty() {
            w.write_paragraphs(info.footer);
        }

        w.into_string()
    }

    /// Returns global options, local options and the help option chained
    /// together.
    fn options(&self) -> impl Iterator<Item = &OptionArgInfo> + Clone {
        self.global_options
            .iter()
            .map(Deref::deref)
            .chain(self.info.options)
            .chain(iter::once(&HELP_OPT))
    }

    /// Returns static and dynamic subcommands chained together, or an empty
    /// vector if no subcommands are defined.
    fn subcommands(&self) -> Vec<&CommandInfo> {
        if let Some(cmds) = &self.info.commands {
            let mut vec = cmds.subcommands.to_vec();
            vec.extend((cmds.dynamic_subcommands)());
            vec
        } else {
            Vec::new()
        }
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
    /// Creates a new [`Help`] generator instance.
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

struct HelpWriter<'a> {
    blank_lines_spacing: &'a str,
    buf: String,
    command_name: &'a str,
    description_indent: usize,
    wrap_width: usize,
}

impl<'a> HelpWriter<'_> {
    #[inline]
    fn write_usage(&'a mut self, title: &str, usage: impl Iterator<Item = &'a str>) {
        let mut line = title.to_string() + " ";

        let usage = iter::once(self.command_name)
            .chain(usage)
            .filter(|s| !s.is_empty());

        self.write_wrapped(&mut line, usage, title.len() + self.command_name.len() + 2);
    }

    fn write_paragraphs(&mut self, text: &str) {
        let text = &text.replace("{command_name}", self.command_name);
        let mut buf = String::new();

        self.write_str(SECTION_SEPARATOR);

        for line in text.split('\n') {
            self.write_wrapped(&mut buf, line.split(' '), 0);
        }
    }

    fn write_section(&mut self, title: &str, descs: impl Iterator<Item = (&'a str, &'a str)>) {
        // NOTE: greedy positional has empty names and description, to be
        // excluded from the Positional Arguments section.
        let mut first = true;
        for desc in descs.filter(|desc| !desc.0.is_empty()) {
            if first {
                self.write_str(SECTION_SEPARATOR);
                self.write_line(title);
                first = false;
            } else {
                self.write_str(self.blank_lines_spacing);
            }
            self.write_description(desc);
        }
    }

    #[inline]
    fn into_string(self) -> String {
        self.buf
    }

    fn write_description(&mut self, (left_col, right_col): (&str, &str)) {
        let mut line = INDENT.to_string();
        line.push_str(left_col);

        if right_col.is_empty() {
            self.write_line_mut(&mut line);
            return;
        }

        if !pad_string(&mut line, self.description_indent) {
            // Start the description on a new line if the flag names already add
            // up to more than `indent`.
            self.write_line_mut(&mut line);
        }

        self.write_wrapped(&mut line, right_col.split(' '), self.description_indent);
    }

    fn write_wrapped<'b>(
        &mut self,
        line: &mut String,
        words: impl Iterator<Item = &'b str>,
        padding: usize,
    ) {
        let mut words = words.peekable();

        while let Some(first_word) = words.next() {
            if padding > 0 && line.is_empty() {
                pad_string(line, padding);
            }
            line.push_str(first_word);

            let mut line_len = chars_count(line);
            'inner: while let Some(&word) = words.peek() {
                let word_len = chars_count(word);

                if (line_len + word_len + 1) > self.wrap_width {
                    self.write_line_mut(line);
                    break 'inner;
                } else {
                    // advance the iterator
                    let _ = words.next();
                    line.push(' ');
                    line.push_str(word);
                    line_len += word_len + 1;
                }
            }
        }
        self.write_line_mut(line);
    }

    fn write_line_mut(&mut self, line: &mut String) {
        self.write_line(line);
        line.truncate(0);
    }

    fn write_line(&mut self, line: &str) {
        self.write_str(line);
        self.write_str("\n");
    }

    #[inline]
    fn write_str(&mut self, s: &str) {
        self.buf.push_str(s);
    }
}

#[inline(never)]
fn chars_count(s: &str) -> usize {
    s.chars().count()
}

/// Pads the given string with spaces until it reaches the given width.
#[inline(never)]
fn pad_string(s: &mut String, width: usize) -> bool {
    let len = chars_count(s);

    if len < width {
        s.extend(iter::repeat(' ').take(width - len));
        true
    } else {
        false
    }
}
