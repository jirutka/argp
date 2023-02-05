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
//! `fn(&str) -> Result<T, String>` using the `from_str_fn` attribute:
//!
//! ```
//! # use argp::FromArgs;
//!
//! /// Goofy thing.
//! #[derive(FromArgs)]
//! struct FiveStruct {
//!     /// Always five.
//!     #[argp(option, from_str_fn(always_five))]
//!     five: usize,
//! }
//!
//! fn always_five(_value: &str) -> Result<usize, String> {
//!     Ok(5)
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
//!     #[cfg(feature = "redact_arg_values")]
//!     fn try_redact_arg_values(
//!         command_name: &[&str],
//!         args: &[&str],
//!     ) -> Option<Result<Vec<String>, EarlyExit>> {
//!         for command in Self::commands() {
//!             if command_name.last() == Some(&command.name) {
//!                 // Process arguments and redact values here.
//!                 if !args.is_empty() {
//!                     return Some(Err(Error::other("Our example dynamic command never takes arguments!").into()));
//!                 }
//!                 return Some(Ok(Vec::new()))
//!             }
//!         }
//!         None
//!     }
//!
//!     fn try_from_args(command_name: &[&str], args: &[&str]) -> Option<Result<Self, EarlyExit>> {
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
mod help;

use std::env;
use std::fmt;
use std::path::Path;
use std::process::exit;
use std::str::FromStr;

pub use crate::error::{Error, MissingRequirements};
pub use crate::help::{CommandInfo, Help, HelpCommands, OptionArgInfo};
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
    ///    argp::EarlyExit::Err(argp::Error::UnknownArgument("lisp".to_owned())),
    /// );
    /// ```
    fn from_args(command_name: &[&str], args: &[&str]) -> Result<Self, EarlyExit> {
        Self::_from_args(command_name, args, None)
    }

    /// Get a String with just the argument names, e.g., options, flags, subcommands, etc, but
    /// without the values of the options and arguments. This can be useful as a means to capture
    /// anonymous usage statistics without revealing the content entered by the end user.
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
    /// #[derive(FromArgs)]
    /// struct ClassroomCmd {
    ///     #[argp(subcommand)]
    ///     subcommands: Subcommands,
    /// }
    ///
    /// #[derive(FromArgs)]
    /// #[argp(subcommand)]
    /// enum Subcommands {
    ///     List(ListCmd),
    ///     Add(AddCmd),
    /// }
    ///
    /// /// List all the classes.
    /// #[derive(FromArgs)]
    /// #[argp(subcommand, name = "list")]
    /// struct ListCmd {
    ///     /// List classes for only this teacher.
    ///     #[argp(option)]
    ///     teacher_name: Option<String>,
    /// }
    ///
    /// /// Add students to a class.
    /// #[derive(FromArgs)]
    /// #[argp(subcommand, name = "add")]
    /// struct AddCmd {
    ///     /// The name of the class's teacher.
    ///     #[argp(option)]
    ///     teacher_name: String,
    ///
    ///     /// Has the class started yet?
    ///     #[argp(switch)]
    ///     started: bool,
    ///
    ///     /// The name of the class.
    ///     #[argp(positional)]
    ///     class_name: String,
    ///
    ///     /// The student names.
    ///     #[argp(positional)]
    ///     students: Vec<String>,
    /// }
    ///
    /// let args = ClassroomCmd::redact_arg_values(
    ///     &["classroom"],
    ///     &["list"],
    /// ).unwrap();
    /// assert_eq!(
    ///     args,
    ///     &[
    ///         "classroom",
    ///         "list",
    ///     ],
    /// );
    ///
    /// let args = ClassroomCmd::redact_arg_values(
    ///     &["classroom"],
    ///     &["list", "--teacher-name", "Smith"],
    /// ).unwrap();
    /// assert_eq!(
    ///    args,
    ///    &[
    ///         "classroom",
    ///         "list",
    ///         "--teacher-name",
    ///     ],
    /// );
    ///
    /// let args = ClassroomCmd::redact_arg_values(
    ///     &["classroom"],
    ///     &["add", "--teacher-name", "Smith", "--started", "Math", "Abe", "Sung"],
    /// ).unwrap();
    /// assert_eq!(
    ///     args,
    ///     &[
    ///         "classroom",
    ///         "add",
    ///         "--teacher-name",
    ///         "--started",
    ///         "class_name",
    ///         "students",
    ///         "students",
    ///     ],
    /// );
    ///
    /// // `ClassroomCmd::redact_arg_values` will error out if passed invalid arguments.
    /// assert_eq!(
    ///     ClassroomCmd::redact_arg_values(&["classroom"], &["add", "--teacher-name"]),
    ///     Err(argp::EarlyExit::Err(argp::Error::MissingArgValue("--teacher-name".to_owned()))),
    /// );
    ///
    /// // `ClassroomCmd::redact_arg_values` will generate help messages.
    /// assert_eq!(
    ///     ClassroomCmd::redact_arg_values(&["classroom"], &["help"]),
    ///     Err(argp::EarlyExit::Help(
    ///         r#"Usage: classroom <command> [<args>]
    ///
    /// Command to manage a classroom.
    ///
    /// Options:
    ///   -h, --help  Show this help message and exit.
    ///
    /// Commands:
    ///   list        List all the classes.
    ///   add         Add students to a class.
    /// "#.to_string())),
    /// );
    /// ```
    #[cfg(feature = "redact_arg_values")]
    fn redact_arg_values(_command_name: &[&str], _args: &[&str]) -> Result<Vec<String>, EarlyExit> {
        Ok(vec!["<<REDACTED>>".into()])
    }

    #[doc(hidden)]
    fn _from_args(
        command_name: &[&str],
        args: &[&str],
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

    /// Perform the function of `FromArgs::redact_arg_values` for this dynamic
    /// command.
    ///
    /// The full list of subcommands, ending with the subcommand that should be
    /// dynamically recognized, is passed in `command_name`. If the command
    /// passed is not recognized, this function should return `None`. Otherwise
    /// it should return `Some`, and the value within the `Some` has the same
    /// semantics as the return of `FromArgs::redact_arg_values`.
    fn try_redact_arg_values(
        _command_name: &[&str],
        _args: &[&str],
    ) -> Option<Result<Vec<String>, EarlyExit>> {
        None
    }

    /// Perform the function of `FromArgs::from_args` for this dynamic command.
    ///
    /// The full list of subcommands, ending with the subcommand that should be
    /// dynamically recognized, is passed in `command_name`. If the command
    /// passed is not recognized, this function should return `None`. Otherwise
    /// it should return `Some`, and the value within the `Some` has the same
    /// semantics as the return of `FromArgs::from_args`.
    fn try_from_args(command_name: &[&str], args: &[&str]) -> Option<Result<Self, EarlyExit>>;
}

/// A `FromArgs` implementation with attached [Help] struct.
pub trait CommandHelp: FromArgs {
    /// Information for generating the help message.
    const HELP: Help;
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

/// Extract the base cmd from a path
fn cmd<'a>(default: &'a str, path: &'a str) -> &'a str {
    Path::new(path).file_name().and_then(|s| s.to_str()).unwrap_or(default)
}

/// Create a `FromArgs` type from the current process's `env::args`.
///
/// This function will exit early from the current process if argument parsing
/// was unsuccessful or if information like `--help` was requested. Error messages will be printed
/// to stderr, and `--help` output to stdout.
pub fn from_env<T: TopLevelCommand>() -> T {
    let strings: Vec<String> = env::args_os()
        .map(|s| s.into_string())
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|arg| {
            eprintln!("Invalid utf8: {}", arg.to_string_lossy());
            exit(1)
        });

    if strings.is_empty() {
        eprintln!("No program name, argv is empty");
        exit(1)
    }

    let cmd = cmd(&strings[0], &strings[0]);
    let strs: Vec<&str> = strings.iter().map(|s| s.as_str()).collect();
    T::from_args(&[cmd], &strs[1..]).unwrap_or_else(|early_exit| {
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
    let strings: Vec<String> = env::args().collect();
    let cmd = cmd(&strings[1], &strings[1]);
    let strs: Vec<&str> = strings.iter().map(|s| s.as_str()).collect();
    T::from_args(&[cmd], &strs[2..]).unwrap_or_else(|early_exit| {
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

/// Types which can be constructed from a single command-line value.
///
/// Any field type declared in a struct that derives `FromArgs` must implement
/// this trait. A blanket implementation exists for types implementing
/// `FromStr<Error: Display>`. Custom types can implement this trait
/// directly.
pub trait FromArgValue: Sized {
    /// Construct the type from a command-line value, returning an error string
    /// on failure.
    fn from_arg_value(value: &str) -> Result<Self, String>;
}

impl<T> FromArgValue for T
where
    T: FromStr,
    T::Err: fmt::Display,
{
    fn from_arg_value(value: &str) -> Result<Self, String> {
        T::from_str(value).map_err(|x| x.to_string())
    }
}

// The following items are all used by the generated code, and should not be considered part
// of this library's public API surface.

#[doc(hidden)]
pub trait ParseFlag {
    fn set_flag(&mut self, arg: &str);
}

impl<T: Flag> ParseFlag for T {
    fn set_flag(&mut self, _arg: &str) {
        <T as Flag>::set_flag(self);
    }
}

#[doc(hidden)]
#[cfg(feature = "redact_arg_values")]
pub struct RedactFlag {
    pub slot: Option<String>,
}

#[cfg(feature = "redact_arg_values")]
impl ParseFlag for RedactFlag {
    fn set_flag(&mut self, arg: &str) {
        self.slot = Some(arg.to_string());
    }
}

// A trait for for slots that reserve space for a value and know how to parse that value
// from a command-line `&str` argument.
//
// This trait is only implemented for the type `ParseValueSlotTy`. This indirection is
// necessary to allow abstracting over `ParseValueSlotTy` instances with different
// generic parameters.
#[doc(hidden)]
pub trait ParseValueSlot {
    fn fill_slot(&mut self, arg: &str, value: &str) -> Result<(), Error>;
}

// The concrete type implementing the `ParseValueSlot` trait.
//
// `T` is the type to be parsed from a single string.
// `Slot` is the type of the container that can hold a value or values of type `T`.
#[doc(hidden)]
pub struct ParseValueSlotTy<Slot, T> {
    // The slot for a parsed value.
    pub slot: Slot,
    // The function to parse the value from a string
    pub parse_func: fn(&str, &str) -> Result<T, String>,
}

// `ParseValueSlotTy<Option<T>, T>` is used as the slot for all non-repeating
// arguments, both optional and required.
impl<T> ParseValueSlot for ParseValueSlotTy<Option<T>, T> {
    fn fill_slot(&mut self, arg: &str, value: &str) -> Result<(), Error> {
        if self.slot.is_some() {
            return Err(Error::DuplicateOption(arg.to_owned()));
        }
        let parsed = (self.parse_func)(arg, value).map_err(|e| Error::ParseArgument {
            arg: arg.to_owned(),
            value: value.to_owned(),
            msg: e,
        })?;
        self.slot = Some(parsed);
        Ok(())
    }
}

// `ParseValueSlotTy<Vec<T>, T>` is used as the slot for repeating arguments.
impl<T> ParseValueSlot for ParseValueSlotTy<Vec<T>, T> {
    fn fill_slot(&mut self, arg: &str, value: &str) -> Result<(), Error> {
        let parsed = (self.parse_func)(arg, value).map_err(|e| Error::ParseArgument {
            arg: arg.to_owned(),
            value: value.to_owned(),
            msg: e,
        })?;
        self.slot.push(parsed);
        Ok(())
    }
}

/// A type which can be the receiver of a `Flag`.
pub trait Flag {
    /// Creates a default instance of the flag value;
    fn default() -> Self
    where
        Self: Sized;

    /// Sets the flag. This function is called when the flag is provided.
    fn set_flag(&mut self);
}

impl Flag for bool {
    fn default() -> Self {
        false
    }
    fn set_flag(&mut self) {
        *self = true;
    }
}

impl Flag for Option<bool> {
    fn default() -> Self {
        None
    }

    fn set_flag(&mut self) {
        *self = Some(true);
    }
}

macro_rules! impl_flag_for_integers {
    ($($ty:ty,)*) => {
        $(
            impl Flag for $ty {
                fn default() -> Self {
                    0
                }
                fn set_flag(&mut self) {
                    *self = self.saturating_add(1);
                }
            }
        )*
    }
}

impl_flag_for_integers![u8, u16, u32, u64, u128, i8, i16, i32, i64, i128,];

/// This function implements argument parsing for structs.
///
/// `cmd_name`: The identifier for the current command.
/// `args`: The command line arguments.
/// `parse_options`: Helper to parse optional arguments.
/// `parse_positionals`: Helper to parse positional arguments.
/// `parse_subcommand`: Helper to parse a subcommand.
/// `help`: The [Help] instance for generating a help message.
#[doc(hidden)]
pub fn parse_struct_args(
    cmd_name: &[&str],
    args: &[&str],
    mut parse_options: ParseStructOptions<'_, '_>,
    mut parse_positionals: ParseStructPositionals<'_>,
    mut parse_subcommand: Option<ParseStructSubCommand<'_>>,
    help: &Help,
) -> Result<(), EarlyExit> {
    let mut help_requested = false;
    let mut remaining_args = args;
    let mut positional_index = 0;
    let mut options_ended = false;

    'parse_args: while let Some(&next_arg) = remaining_args.first() {
        remaining_args = &remaining_args[1..];
        if (next_arg == "--help" || next_arg == "help" || next_arg == "-h") && !options_ended {
            help_requested = true;
            continue;
        }

        if next_arg.starts_with('-') && !options_ended {
            if next_arg == "--" {
                options_ended = true;
                continue;
            }

            if help_requested {
                return Err(Error::OptionsAfterHelp.into());
            }

            parse_options.parse(next_arg, &mut remaining_args)?;

            continue;
        }

        if let Some(ref mut parse_subcommand) = parse_subcommand {
            if parse_subcommand.parse(
                help_requested,
                cmd_name,
                next_arg,
                remaining_args,
                &mut parse_options,
            )? {
                // Unset `help`, since we handled it in the subcommand
                help_requested = false;
                break 'parse_args;
            }
        }

        options_ended |= parse_positionals.parse(&mut positional_index, next_arg)?;
    }

    if help_requested {
        let global_options = parse_options.parent.map_or_else(Vec::new, |p| p.global_options());

        Err(EarlyExit::Help(help.generate(cmd_name, &global_options)))
    } else {
        Ok(())
    }
}

#[doc(hidden)]
pub struct ParseStructOptions<'a, 'p> {
    /// A mapping from option string literals to the entry
    /// in the output table. This may contain multiple entries mapping to
    /// the same location in the table if both a short and long version
    /// of the option exist (`-z` and `--zoo`).
    pub arg_to_slot: &'static [(&'static str, usize)],

    /// The storage for argument output data.
    pub slots: &'a mut [ParseStructOption<'a>],

    /// A boolean flag for each element of the `slots` slice that specifies
    /// whether the option(s) associated with the slot is a global option.
    pub slots_global: &'static [bool],

    /// A reference to the [Help] struct in the associated [FromArgs].
    /// This is used to collect global options for generating a help message.
    pub help: &'static Help,

    /// If this struct represents options of a subcommand, then `parent` is an
    /// indirect reference to the previous [ParseStructOptions] in the chain,
    /// used for parsing global options.
    pub parent: Option<&'p mut dyn ParseGlobalOptions>,
}

#[doc(hidden)]
pub trait ParseGlobalOptions {
    /// Parse a global command-line option. If the option is not found in _self_,
    /// it recursively calls this function on the parent. If the option is
    /// still not found, it returns `None`.
    ///
    /// - `arg`: the current option argument being parsed (e.g. `--foo`).
    /// - `remaining_args`: the remaining command line arguments. This slice
    ///    will be advanced forwards if the option takes a value argument.
    fn try_parse_global(
        &mut self,
        arg: &str,
        remaining_args: &mut &[&str],
    ) -> Option<Result<(), Error>>;

    /// Returns a vector representing global options specified on this instance
    /// and recursively on the parent. This is used for generating a help
    /// message.
    fn global_options<'a>(&self) -> Vec<&'a OptionArgInfo>;
}

impl<'a, 'p> ParseStructOptions<'a, 'p> {
    /// Parses a command-line option. If the option is not found in this
    /// instance, it tries to parse it as a global option in the parent
    /// instance, recursively. If it's not found even there, returns
    /// `Err("Unrecognized argument: {arg}")`.
    ///
    /// - `arg`: The current option argument being parsed (e.g. `--foo`).
    /// - `remaining_args`: The remaining command line arguments. This slice
    ///    will be advanced forwards if the option takes a value argument.
    fn parse(&mut self, arg: &str, remaining_args: &mut &[&str]) -> Result<(), Error> {
        match self.arg_to_slot.iter().find(|(name, _)| *name == arg) {
            Some((_, pos)) => Self::fill_slot(&mut self.slots[*pos], arg, remaining_args),
            None => self
                .try_parse_global(arg, remaining_args)
                .unwrap_or_else(|| Err(Error::UnknownArgument(arg.to_owned()))),
        }
    }

    fn fill_slot(
        slot: &mut ParseStructOption<'a>,
        arg: &str,
        remaining_args: &mut &[&str],
    ) -> Result<(), Error> {
        match slot {
            ParseStructOption::Flag(ref mut b) => b.set_flag(arg),
            ParseStructOption::Value(ref mut pvs) => {
                let value =
                    remaining_args.first().ok_or_else(|| Error::MissingArgValue(arg.to_owned()))?;
                *remaining_args = &remaining_args[1..];
                pvs.fill_slot(arg, value)?;
            }
        }
        Ok(())
    }
}

impl<'a, 'p> ParseGlobalOptions for ParseStructOptions<'a, 'p> {
    fn try_parse_global(
        &mut self,
        arg: &str,
        remaining_args: &mut &[&str],
    ) -> Option<Result<(), Error>> {
        self.arg_to_slot
            .iter()
            .find(|(name, pos)| *name == arg && self.slots_global[*pos])
            .map(|(_, pos)| Self::fill_slot(&mut self.slots[*pos], arg, remaining_args))
            .or_else(|| self.parent.as_mut().and_then(|p| p.try_parse_global(arg, remaining_args)))
    }

    fn global_options<'b>(&self) -> Vec<&'b OptionArgInfo> {
        let mut opts = self.parent.as_ref().map_or_else(Vec::new, |p| p.global_options());
        opts.extend(self.help.options.iter().filter(|o| o.global));
        opts
    }
}

// `--` or `-` options, including a mutable reference to their value.
#[doc(hidden)]
pub enum ParseStructOption<'a> {
    // A flag which is set to `true` when provided.
    Flag(&'a mut dyn ParseFlag),
    // A value which is parsed from the string following the `--` argument,
    // e.g. `--foo bar`.
    Value(&'a mut dyn ParseValueSlot),
}

#[doc(hidden)]
pub struct ParseStructPositionals<'a> {
    pub positionals: &'a mut [ParseStructPositional<'a>],
    pub last_is_repeating: bool,
    pub last_is_greedy: bool,
}

impl<'a> ParseStructPositionals<'a> {
    /// Parse the next positional argument.
    ///
    /// `arg`: the argument supplied by the user.
    ///
    /// Returns true if non-positional argument parsing should stop
    /// after this one.
    fn parse(&mut self, index: &mut usize, arg: &str) -> Result<bool, Error> {
        if *index < self.positionals.len() {
            self.positionals[*index].parse(arg)?;

            if self.last_is_repeating && *index == self.positionals.len() - 1 {
                // Don't increment position if we're at the last arg
                // *and* the last arg is repeating. If it's also remainder,
                // halt non-option processing after this.
                Ok(self.last_is_greedy)
            } else {
                // If it is repeating, though, increment the index and continue
                // processing options.
                *index += 1;
                Ok(false)
            }
        } else {
            Err(Error::UnknownArgument(arg.to_owned()))
        }
    }
}

#[doc(hidden)]
pub struct ParseStructPositional<'a> {
    // The positional's name
    pub name: &'static str,

    // The function to parse the positional.
    pub slot: &'a mut dyn ParseValueSlot,
}

impl<'a> ParseStructPositional<'a> {
    /// Parse a positional argument.
    ///
    /// `arg`: the argument supplied by the user.
    fn parse(&mut self, arg: &str) -> Result<(), Error> {
        self.slot.fill_slot(self.name, arg)
    }
}

// A type to simplify parsing struct subcommands.
//
// This indirection is necessary to allow abstracting over `FromArgs` instances with different
// generic parameters.
#[doc(hidden)]
pub struct ParseStructSubCommand<'a> {
    // The subcommand commands
    pub subcommands: &'static [&'static CommandInfo],

    pub dynamic_subcommands: &'a [&'static CommandInfo],

    // The function to parse the subcommand arguments.
    #[allow(clippy::type_complexity)]
    pub parse_func: &'a mut dyn FnMut(
        &[&str],
        &[&str],
        Option<&mut dyn ParseGlobalOptions>,
    ) -> Result<(), EarlyExit>,
}

impl<'a> ParseStructSubCommand<'a> {
    fn parse(
        &mut self,
        help: bool,
        cmd_name: &[&str],
        arg: &str,
        remaining_args: &[&str],
        parse_global_opts: &mut dyn ParseGlobalOptions,
    ) -> Result<bool, EarlyExit> {
        for subcommand in self.subcommands.iter().chain(self.dynamic_subcommands.iter()) {
            if subcommand.name == arg {
                let mut command = cmd_name.to_owned();
                command.push(subcommand.name);
                let prepended_help;
                let remaining_args = if help {
                    prepended_help = prepend_help(remaining_args);
                    &prepended_help
                } else {
                    remaining_args
                };

                (self.parse_func)(&command, remaining_args, Some(parse_global_opts))?;

                return Ok(true);
            }
        }

        Ok(false)
    }
}

// Prepend `help` to a list of arguments.
// This is used to pass the `help` argument on to subcommands.
fn prepend_help<'a>(args: &[&'a str]) -> Vec<&'a str> {
    [&["help"], args].concat()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_cmd_extraction() {
        let expected = "test_cmd";
        let path = format!("/tmp/{}", expected);
        let cmd = cmd(&path, &path);
        assert_eq!(expected, cmd);
    }
}
