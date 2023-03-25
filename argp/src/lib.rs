// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2023 Jakub Jirutka <jakub@jirutka.cz>
// SPDX-FileCopyrightText: 2020 Google LLC

//! Derive-based argument parsing optimized for code size and flexibility.
//!
//! The public API of this library consists primarily of the `FromArgs`
//! derive and the `from_env` function, which can be used to produce
//! a top-level `FromArgs` type from the current program's command-line
//! arguments.
//!
//! ## Basic Example
//!
//! ```rust,no_run
//! use argp::FromArgs;
//!
//! /// Reach new heights.
//! #[derive(FromArgs)]
//! struct GoUp {
//!     /// Whether or not to jump.
//!     #[argp(switch, short = 'j')]
//!     jump: bool,
//!
//!     /// How high to go.
//!     #[argp(option)]
//!     height: usize,
//!
//!     /// An optional nickname for the pilot.
//!     #[argp(option)]
//!     pilot_nickname: Option<String>,
//! }
//!
//! let up: GoUp = argp::from_env();
//! ```
//!
//! `./some_bin --help` will then output the following:
//!
//! ```bash
//! Usage: cmdname [-j] --height <height> [--pilot-nickname <pilot-nickname>]
//!
//! Reach new heights.
//!
//! Options:
//!   -j, --jump        Whether or not to jump.
//!   --height          How high to go.
//!   --pilot-nickname  An optional nickname for the pilot.
//!   --help            Show this help message and exit.
//! ```
//!
//! The resulting program can then be used in any of these ways:
//! - `./some_bin --height 5`
//! - `./some_bin -j --height 5`
//! - `./some_bin --jump --height 5 --pilot-nickname Wes`
//!
//! Switches, like `jump`, are optional and will be set to true if provided.
//!
//! Options, like `height` and `pilot_nickname`, can be either required,
//! optional, or repeating, depending on whether they are contained in an
//! `Option` or a `Vec`. Default values can be provided using the
//! `#[argp(default = "<your_code_here>")]` attribute, and in this case an
//! option is treated as optional.
//!
//! ```rust
//! use argp::FromArgs;
//!
//! fn default_height() -> usize {
//!     5
//! }
//!
//! /// Reach new heights.
//! #[derive(FromArgs)]
//! struct GoUp {
//!     /// An optional nickname for the pilot.
//!     #[argp(option)]
//!     pilot_nickname: Option<String>,
//!
//!     /// An optional height.
//!     #[argp(option, default = "default_height()")]
//!     height: usize,
//!
//!     /// An optional direction which is "up" by default.
//!     #[argp(option, default = "String::from(\"only up\")")]
//!     direction: String,
//! }
//!
//! fn main() {
//!     let up: GoUp = argp::from_env();
//! }
//! ```
//!
//! Custom option types can be deserialized so long as they implement the
//! `FromArgValue` trait (automatically implemented for all `FromStr` types).
//! If more customized parsing is required, you can supply a custom
//! `fn(&str) -> Result<T, String>` using the `from_str_fn` attribute, or
//! `fn(&OsStr) -> Result<T, String>` using the `from_os_str_fn` attribute:
//!
//! ```
//! # use argp::FromArgs;
//! # use std::ffi::OsStr;
//! # use std::path::PathBuf;
//!
//! /// Goofy thing.
//! #[derive(FromArgs)]
//! struct FineStruct {
//!     /// Always five.
//!     #[argp(option, from_str_fn(always_five))]
//!     five: usize,
//!
//!     /// File path.
//!     #[argp(option, from_os_str_fn(convert_path))]
//!     path: PathBuf,
//! }
//!
//! fn always_five(_value: &str) -> Result<usize, String> {
//!     Ok(5)
//! }
//!
//! fn convert_path(value: &OsStr) -> Result<PathBuf, String> {
//!     Ok(PathBuf::from("/tmp").join(value))
//! }
//! ```
//!
//! Positional arguments can be declared using `#[argp(positional)]`.
//! These arguments will be parsed in order of their declaration in
//! the structure:
//!
//! ```rust
//! # use argp::FromArgs;
//!
//! /// A command with positional arguments.
//! #[derive(FromArgs, PartialEq, Debug)]
//! struct WithPositional {
//!     #[argp(positional)]
//!     first: String,
//! }
//! ```
//!
//! The last positional argument may include a default, or be wrapped in
//! `Option` or `Vec` to indicate an optional or repeating positional argument.
//!
//! If your final positional argument has the `greedy` option on it, it will consume
//! any arguments after it as if a `--` were placed before the first argument to
//! match the greedy positional:
//!
//! ```rust
//! # use argp::FromArgs;
//!
//! /// A command with a greedy positional argument at the end.
//! #[derive(FromArgs, PartialEq, Debug)]
//! struct WithGreedyPositional {
//!     /// Some stuff.
//!     #[argp(option)]
//!     stuff: Option<String>,
//!     #[argp(positional, greedy)]
//!     all_the_rest: Vec<String>,
//! }
//! ```
//!
//! Now if you pass `--stuff Something` after a positional argument, it will
//! be consumed by `all_the_rest` instead of setting the `stuff` field.
//!
//! Note that `all_the_rest` won't be listed as a positional argument in the
//! long text part of help output (and it will be listed at the end of the usage
//! line as `[all_the_rest...]`), and it's up to the caller to append any
//! extra help output for the meaning of the captured arguments. This is to
//! enable situations where some amount of argument processing needs to happen
//! before the rest of the arguments can be interpreted, and shouldn't be used
//! for regular use as it might be confusing.
//!
//! ## Subcommands
//!
//! Subcommands are also supported. To use a subcommand, declare a separate
//! `FromArgs` type for each subcommand as well as an enum that cases
//! over each command:
//!
//! ```rust
//! # use argp::FromArgs;
//!
//! /// Top-level command.
//! #[derive(FromArgs, PartialEq, Debug)]
//! struct TopLevel {
//!     /// Be verbose.
//!     #[argp(switch, short = 'v', global)]
//!     verbose: bool,
//!
//!     /// Run locally.
//!     #[argp(switch)]
//!     quiet: bool,
//!
//!     #[argp(subcommand)]
//!     nested: MySubCommandEnum,
//! }
//!
//! #[derive(FromArgs, PartialEq, Debug)]
//! #[argp(subcommand)]
//! enum MySubCommandEnum {
//!     One(SubCommandOne),
//!     Two(SubCommandTwo),
//! }
//!
//! /// First subcommand.
//! #[derive(FromArgs, PartialEq, Debug)]
//! #[argp(subcommand, name = "one")]
//! struct SubCommandOne {
//!     /// How many x.
//!     #[argp(option)]
//!     x: usize,
//! }
//!
//! /// Second subcommand.
//! #[derive(FromArgs, PartialEq, Debug)]
//! #[argp(subcommand, name = "two")]
//! struct SubCommandTwo {
//!     /// Whether to fooey.
//!     #[argp(switch)]
//!     fooey: bool,
//! }
//! ```
//!
//! Normally the options specified in `TopLevel` must be placed before the
//! subcommand name, e.g. `./some_bin --quiet one --x 42` will work, but
//! `./some_bin one --quiet --x 42` won't. To allow an option from a higher
//! level to be used at a lower level (in subcommands), you can specify the
//! `global` attribute to the option (`--verbose` in the example above).
//!
//! Global options only propagate down, not up (to parent commands), but their
//! values are propagated back up to the parent once a user has used them. In
//! effect, this means that you should define all global arguments at the top
//! level, but it doesn't matter where the user uses the global argument.
//!
//! You can also discover subcommands dynamically at runtime. To do this,
//! declare subcommands as usual and add a variant to the enum with the
//! `dynamic` attribute. Instead of deriving `FromArgs`, the value inside the
//! dynamic variant should implement `DynamicSubCommand`.
//!
//! ```rust
//! # use argp::CommandInfo;
//! # use argp::DynamicSubCommand;
//! # use argp::EarlyExit;
//! # use argp::Error;
//! # use argp::FromArgs;
//! # use once_cell::sync::OnceCell;
//! # use std::ffi::OsStr;
//!
//! /// Top-level command.
//! #[derive(FromArgs, PartialEq, Debug)]
//! struct TopLevel {
//!     #[argp(subcommand)]
//!     nested: MySubCommandEnum,
//! }
//!
//! #[derive(FromArgs, PartialEq, Debug)]
//! #[argp(subcommand)]
//! enum MySubCommandEnum {
//!     Normal(NormalSubCommand),
//!     #[argp(dynamic)]
//!     Dynamic(Dynamic),
//! }
//!
//! /// Normal subcommand.
//! #[derive(FromArgs, PartialEq, Debug)]
//! #[argp(subcommand, name = "normal")]
//! struct NormalSubCommand {
//!     /// How many x.
//!     #[argp(option)]
//!     x: usize,
//! }
//!
//! /// Dynamic subcommand.
//! #[derive(PartialEq, Debug)]
//! struct Dynamic {
//!     name: String
//! }
//!
//! impl DynamicSubCommand for Dynamic {
//!     fn commands() -> &'static [&'static CommandInfo] {
//!         static RET: OnceCell<Vec<&'static CommandInfo>> = OnceCell::new();
//!         RET.get_or_init(|| {
//!             let mut commands = Vec::new();
//!
//!             // argp needs the `CommandInfo` structs we generate to be valid
//!             // for the static lifetime. We can allocate the structures on
//!             // the heap with `Box::new` and use `Box::leak` to get a static
//!             // reference to them. We could also just use a constant
//!             // reference, but only because this is a synthetic example; the
//!             // point of using dynamic commands is to have commands you
//!             // don't know about until runtime!
//!             commands.push(&*Box::leak(Box::new(CommandInfo {
//!                 name: "dynamic_command",
//!                 description: "A dynamic command",
//!             })));
//!
//!             commands
//!         })
//!     }
//!
//!     fn try_from_args(command_name: &[&str], args: &[&OsStr]) -> Option<Result<Self, EarlyExit>> {
//!         for command in Self::commands() {
//!             if command_name.last() == Some(&command.name) {
//!                 if !args.is_empty() {
//!                     return Some(Err(Error::other("Our example dynamic command never takes arguments!").into()));
//!                 }
//!                 return Some(Ok(Dynamic { name: command.name.to_string() }))
//!             }
//!         }
//!         None
//!     }
//! }
//! ```
//!
//! ## Help message
//!
//! Programs that are run from an environment such as cargo may find it
//! useful to have positional arguments present in the structure but
//! omitted from the usage output. This can be accomplished by adding
//! the `hidden_help` attribute to that argument:
//!
//! ```rust
//! # use argp::FromArgs;
//!
//! /// Cargo arguments
//! #[derive(FromArgs)]
//! struct CargoArgs {
//!     // Cargo puts the command name invoked into the first argument,
//!     // so we don't want this argument to show up in the usage text.
//!     #[argp(positional, hidden_help)]
//!     command: String,
//!     /// An option used for internal debugging.
//!     #[argp(option, hidden_help)]
//!     internal_debugging: String,
//!     #[argp(positional)]
//!     real_first_arg: String,
//! }
//! ```

#![deny(missing_docs)]

mod error;
pub mod help;
pub mod parser;

use std::borrow::Cow;
use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::path::Path;
use std::process::exit;
use std::str::FromStr;

use crate::help::Help;
use crate::parser::ParseGlobalOptions;

pub use crate::error::{Error, MissingRequirements};
pub use crate::help::CommandInfo;
pub use argp_derive::FromArgs;

/// Types which can be constructed from a set of command-line arguments.
pub trait FromArgs: Sized {
    /// Construct the type from an input set of arguments.
    ///
    /// The first argument `command_name` is the identifier for the current command. In most cases,
    /// users should only pass in a single item for the command name, which typically comes from
    /// the first item from `std::env::args()`. Implementations however should append the
    /// subcommand name in when recursively calling [FromArgs::from_args] for subcommands. This
    /// allows `argp` to generate correct subcommand help strings.
    ///
    /// The second argument `args` is the rest of the command line arguments.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use argp::FromArgs;
    ///
    /// /// Command to manage a classroom.
    /// #[derive(Debug, PartialEq, FromArgs)]
    /// struct ClassroomCmd {
    ///     #[argp(subcommand)]
    ///     subcommands: Subcommands,
    /// }
    ///
    /// #[derive(Debug, PartialEq, FromArgs)]
    /// #[argp(subcommand)]
    /// enum Subcommands {
    ///     List(ListCmd),
    ///     Add(AddCmd),
    /// }
    ///
    /// /// List all the classes.
    /// #[derive(Debug, PartialEq, FromArgs)]
    /// #[argp(subcommand, name = "list")]
    /// struct ListCmd {
    ///     /// List classes for only this teacher.
    ///     #[argp(option, arg_name = "name")]
    ///     teacher_name: Option<String>,
    /// }
    ///
    /// /// Add students to a class.
    /// #[derive(Debug, PartialEq, FromArgs)]
    /// #[argp(subcommand, name = "add")]
    /// struct AddCmd {
    ///     /// The name of the class's teacher.
    ///     #[argp(option)]
    ///     teacher_name: String,
    ///
    ///     /// The name of the class.
    ///     #[argp(positional)]
    ///     class_name: String,
    /// }
    ///
    /// let args = ClassroomCmd::from_args(
    ///     &["classroom"],
    ///     &["list", "--teacher-name", "Smith"],
    /// ).unwrap();
    /// assert_eq!(
    ///    args,
    ///     ClassroomCmd {
    ///         subcommands: Subcommands::List(ListCmd {
    ///             teacher_name: Some("Smith".to_string()),
    ///         })
    ///     },
    /// );
    ///
    /// // Help returns an error with `EarlyExit::Help`.
    /// let early_exit = ClassroomCmd::from_args(
    ///     &["classroom"],
    ///     &["help"],
    /// ).unwrap_err();
    /// assert_eq!(
    ///     early_exit,
    ///     argp::EarlyExit::Help(
    ///        r#"Usage: classroom <command> [<args>]
    ///
    /// Command to manage a classroom.
    ///
    /// Options:
    ///   -h, --help  Show this help message and exit.
    ///
    /// Commands:
    ///   list        List all the classes.
    ///   add         Add students to a class.
    /// "#.to_owned()),
    /// );
    ///
    /// // Help works with subcommands.
    /// let early_exit = ClassroomCmd::from_args(
    ///     &["classroom"],
    ///     &["list", "help"],
    /// ).unwrap_err();
    /// assert_eq!(
    ///     early_exit,
    ///     argp::EarlyExit::Help(
    ///        r#"Usage: classroom list [--teacher-name <name>]
    ///
    /// List all the classes.
    ///
    /// Options:
    ///       --teacher-name <name>  List classes for only this teacher.
    ///   -h, --help                 Show this help message and exit.
    /// "#.to_owned()),
    /// );
    ///
    /// // Incorrect arguments will error out.
    /// let err = ClassroomCmd::from_args(
    ///     &["classroom"],
    ///     &["lisp"],
    /// ).unwrap_err();
    /// assert_eq!(
    ///    err,
    ///    argp::EarlyExit::Err(argp::Error::UnknownArgument("lisp".into())),
    /// );
    /// ```
    fn from_args<S: AsRef<OsStr>>(command_name: &[&str], args: &[S]) -> Result<Self, EarlyExit> {
        let args: Vec<_> = args.iter().map(AsRef::as_ref).collect();
        Self::_from_args(command_name, &args, None)
    }

    #[doc(hidden)]
    fn _from_args(
        command_name: &[&str],
        args: &[&OsStr],
        parent: Option<&mut dyn ParseGlobalOptions>,
    ) -> Result<Self, EarlyExit>;
}

/// A top-level `FromArgs` implementation that is not a subcommand.
pub trait TopLevelCommand: FromArgs {}

/// A `FromArgs` implementation that can parse into one or more subcommands.
pub trait SubCommands: FromArgs {
    /// Info for the commands.
    const COMMANDS: &'static [&'static CommandInfo];

    /// Get a list of commands that are discovered at runtime.
    fn dynamic_commands() -> &'static [&'static CommandInfo] {
        &[]
    }
}

/// A `FromArgs` implementation that represents a single subcommand.
pub trait SubCommand: FromArgs {
    /// Information about the subcommand.
    const COMMAND: &'static CommandInfo;
}

impl<T: SubCommand> SubCommands for T {
    const COMMANDS: &'static [&'static CommandInfo] = &[T::COMMAND];
}

/// Trait implemented by values returned from a dynamic subcommand handler.
pub trait DynamicSubCommand: Sized {
    /// Info about supported subcommands.
    fn commands() -> &'static [&'static CommandInfo];

    /// Perform the function of `FromArgs::from_args` for this dynamic command.
    ///
    /// The full list of subcommands, ending with the subcommand that should be
    /// dynamically recognized, is passed in `command_name`. If the command
    /// passed is not recognized, this function should return `None`. Otherwise
    /// it should return `Some`, and the value within the `Some` has the same
    /// semantics as the return of `FromArgs::from_args`.
    fn try_from_args(command_name: &[&str], args: &[&OsStr]) -> Option<Result<Self, EarlyExit>>;
}

/// A `FromArgs` implementation with attached [Help] struct.
pub trait CommandHelp: FromArgs {
    /// Information for generating the help message.
    const HELP: Help;
}

/// Types which can be constructed from a single command-line value.
///
/// Any field type declared in a struct that derives `FromArgs` must implement
/// this trait. A blanket implementation exists for types implementing
/// `FromStr<Error: Display>`. Custom types can implement this trait
/// directly.
pub trait FromArgValue: Sized {
    /// Construct the type from a command-line value, returning an error string
    /// on failure.
    fn from_arg_value(value: &OsStr) -> Result<Self, String>;
}

// TODO: rework
impl<T> FromArgValue for T
where
    T: FromStr,
    T::Err: fmt::Display,
{
    fn from_arg_value(value: &OsStr) -> Result<Self, String> {
        value
            .to_str()
            .ok_or("not a valid UTF-8 string".to_owned())
            .and_then(|s| T::from_str(s).map_err(|e| e.to_string()))
    }
}

/// Information to display to the user about why a `FromArgs` construction exited early.
///
/// This can occur due to either failed parsing or a flag like `--help`.
#[derive(Debug, PartialEq)]
pub enum EarlyExit {
    /// Early exit and display the error message.
    Err(Error),

    /// Early exit and display the help message.
    Help(String),
}

impl fmt::Display for EarlyExit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EarlyExit::Err(err) => err.fmt(f),
            EarlyExit::Help(output) => output.fmt(f),
        }
    }
}

impl From<Error> for EarlyExit {
    fn from(value: Error) -> Self {
        Self::Err(value)
    }
}

/// Create a `FromArgs` type from the current process's `env::args`.
///
/// This function will exit early from the current process if argument parsing
/// was unsuccessful or if information like `--help` was requested. Error messages will be printed
/// to stderr, and `--help` output to stdout.
pub fn from_env<T: TopLevelCommand>() -> T {
    let args: Vec<_> = env::args_os().collect();
    if args.is_empty() {
        eprintln!("No program name, argv is empty");
        exit(1)
    }
    let cmd = basename(&args[0]);

    T::from_args(&[&cmd], &args[1..]).unwrap_or_else(|early_exit| {
        exit(match early_exit {
            EarlyExit::Help(output) => {
                println!("{}", output);
                0
            }
            EarlyExit::Err(err) => {
                eprintln!("{}\nRun {} --help for more information.", err, cmd);
                1
            }
        })
    })
}

/// Create a `FromArgs` type from the current process's `env::args`.
///
/// This special cases usages where argp is being used in an environment where cargo is
/// driving the build. We skip the second env variable.
///
/// This function will exit early from the current process if argument parsing
/// was unsuccessful or if information like `--help` was requested. Error messages will be printed
/// to stderr, and `--help` output to stdout.
pub fn cargo_from_env<T: TopLevelCommand>() -> T {
    let args: Vec<_> = env::args_os().collect();
    let cmd = basename(&args[1]);

    T::from_args(&[&cmd], &args[2..]).unwrap_or_else(|early_exit| {
        exit(match early_exit {
            EarlyExit::Help(output) => {
                println!("{}", output);
                0
            }
            EarlyExit::Err(err) => {
                eprintln!("{}\nRun --help for more information.", err);
                1
            }
        })
    })
}

/// Extracts the base command from a path.
fn basename(path: &OsStr) -> Cow<'_, str> {
    Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .map(Cow::from)
        .unwrap_or_else(|| path.to_string_lossy())
}

#[cfg(test)]
mod test {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn test_basename() {
        let expected = "test_cmd";
        let path = OsString::from(format!("/tmp/{}", expected));
        let cmd = basename(&path);
        assert_eq!(expected, cmd);
    }
}
