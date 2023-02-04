// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2023 Jakub Jirutka <jakub@jirutka.cz>

use std::fmt::{self, Write as _};

/// The error type for the argp parser.
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Duplicate value for a non-repeating option. The contained `String` is
    /// the option name (e.g. `--foo`).
    DuplicateOption(String),

    /// No value provided for the specified option.
    MissingArgValue(String),

    /// Missing required positional argument(s), option(s) or subcommand(s).
    MissingRequirements(MissingRequirements),

    /// Trailing options after the `help` subcommand.
    OptionsAfterHelp,

    /// Error parsing the given value for the positional argument or option.
    ParseArgument {
        /// The positional argument or option.
        arg: String,
        /// The given value that failed to be parsed.
        value: String,
        /// The error message from the value parser.
        msg: String,
    },

    /// Unknown argument.
    UnknownArgument(String),

    /// Any other error.
    Other(String),
}

impl Error {
    /// A convenient method for creating [Error::Other].
    #[inline]
    pub fn other<S: ToString>(msg: S) -> Self {
        Self::Other(msg.to_string())
    }
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Error::*;

        match &self {
            DuplicateOption(_) => write!(f, "duplicate values provided"),
            MissingArgValue(arg) => write!(f, "No value provided for option '{}'.", arg),
            MissingRequirements(req) => req.fmt(f),
            OptionsAfterHelp => write!(f, "Trailing options are not allowed after `help`."),
            ParseArgument { arg, value, msg } => {
                write!(f, "Error parsing argument '{}' with value '{}': {}", arg, value, msg)
            }
            UnknownArgument(arg) => write!(f, "Unrecognized argument: {}", arg),
            Other(msg) => msg.fmt(f),
        }
    }
}

// An error string builder to report missing required options and subcommands.
#[doc(hidden)]
#[derive(Debug, Default, PartialEq)]
pub struct MissingRequirements {
    options: Vec<&'static str>,
    subcommands: Option<Vec<&'static str>>,
    positional_args: Vec<&'static str>,
}

impl MissingRequirements {
    // Add a missing required option.
    #[doc(hidden)]
    pub fn missing_option(&mut self, name: &'static str) {
        self.options.push(name)
    }

    // Add a missing required subcommand.
    #[doc(hidden)]
    pub fn missing_subcommands(&mut self, commands: impl Iterator<Item = &'static str>) {
        self.subcommands = Some(commands.collect());
    }

    // Add a missing positional argument.
    #[doc(hidden)]
    pub fn missing_positional_arg(&mut self, name: &'static str) {
        self.positional_args.push(name)
    }

    // If any missing options or subcommands were provided, returns an error string
    // describing the missing args.
    #[doc(hidden)]
    pub fn err_on_any(self) -> Result<(), Error> {
        if self.options.is_empty() && self.subcommands.is_none() && self.positional_args.is_empty()
        {
            Ok(())
        } else {
            Err(Error::MissingRequirements(self))
        }
    }
}

const NEWLINE_INDENT: &str = "\n    ";

impl fmt::Display for MissingRequirements {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.positional_args.is_empty() {
            f.write_str("Required positional arguments not provided:")?;
            for arg in &self.positional_args {
                f.write_str(NEWLINE_INDENT)?;
                f.write_str(arg)?;
            }
        }

        if !self.options.is_empty() {
            if !self.positional_args.is_empty() {
                f.write_char('\n')?;
            }
            f.write_str("Required options not provided:")?;
            for option in &self.options {
                f.write_str(NEWLINE_INDENT)?;
                f.write_str(option)?;
            }
        }

        if let Some(missing_subcommands) = &self.subcommands {
            if !self.options.is_empty() {
                f.write_char('\n')?;
            }
            f.write_str("One of the following subcommands must be present:")?;
            f.write_str(NEWLINE_INDENT)?;
            f.write_str("help")?;
            for subcommand in missing_subcommands {
                f.write_str(NEWLINE_INDENT)?;
                f.write_str(subcommand)?;
            }
        }

        f.write_char('\n')
    }
}
