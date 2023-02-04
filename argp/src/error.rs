// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2023 Jakub Jirutka <jakub@jirutka.cz>

use std::fmt;

use crate::MissingRequirements;

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
