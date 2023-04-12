// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2023 Jakub Jirutka <jakub@jirutka.cz>
// SPDX-FileCopyrightText: 2020 Google LLC

//! Items in this module are all used by the generated code, and should not be
//! considered part of this library's public API surface.

use std::ffi::{OsStr, OsString};

use crate::error::Error;
use crate::help::{CommandInfo, HelpInfo, OptionArgInfo};
use crate::EarlyExit;

/// This function implements argument parsing for structs.
///
/// - `cmd_name`: The identifier for the current command.
/// - `args`: The command line arguments.
/// - `parse_options`: Helper to parse optional arguments.
/// - `parse_positionals`: Helper to parse positional arguments.
/// - `parse_subcommand`: Helper to parse a subcommand.
/// - `help`: The [`Help`] instance for generating a help message.
#[doc(hidden)]
pub fn parse_struct_args(
    cmd_name: &[&str],
    args: &[&OsStr],
    mut parse_options: ParseStructOptions<'_, '_>,
    mut parse_positionals: ParseStructPositionals<'_>,
    mut parse_subcommand: Option<ParseStructSubCommand<'_>>,
    help: &'static HelpInfo,
) -> Result<(), EarlyExit> {
    let mut help_requested = false;
    let mut help_cmd = false;
    let mut remaining_args = args;
    let mut positional_index = 0;
    let mut options_ended = false;

    'parse_args: while let Some(&next_arg_os) = remaining_args.first() {
        remaining_args = &remaining_args[1..];
        let next_arg = next_arg_os.to_str().unwrap_or("");

        if matches!(next_arg, "--help" | "-h" | "help") && !options_ended {
            help_requested = true;
            help_cmd = next_arg_os == "help";
            continue;
        }

        if next_arg.starts_with('-') && !options_ended {
            if next_arg_os == "--" {
                options_ended = true;
                continue;
            }

            if help_cmd {
                return Err(Error::OptionsAfterHelp.into());
            }

            // Handle combined short options; `-ab` is parsed as `-a -b`,
            // `-an 5` as `-a -n 5`, but `-na 5` would fail.
            if next_arg.len() > 2 && &next_arg[1..2] != "-" {
                let mut chars = next_arg[1..].chars().peekable();

                while let Some(short) = chars.next() {
                    // Only the last option can accept a value.
                    if chars.peek().is_some() {
                        parse_options.parse(&format!("-{}", short), &mut (&[] as &[&OsStr]))?;
                    } else {
                        parse_options.parse(&format!("-{}", short), &mut remaining_args)?;
                    }
                }
            } else {
                parse_options.parse(next_arg, &mut remaining_args)?;
            }

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

        options_ended |= parse_positionals.parse(&mut positional_index, next_arg_os)?;
    }

    if help_requested {
        let global_options = parse_options
            .parent
            .map_or_else(Vec::new, |p| p.global_options());

        Err(EarlyExit::Help(help.help(cmd_name.join(" "), global_options)))
    } else {
        Ok(())
    }
}

#[doc(hidden)]
pub struct ParseStructOptions<'a, 'p> {
    /// A mapping from option string literals to the entry in the output table.
    /// This may contain multiple entries mapping to the same location in the
    /// table if both a short and long version of the option exist (`-z` and
    /// `--zoo`).
    pub arg_to_slot: &'static [(&'static str, usize)],

    /// The storage for argument output data.
    pub slots: &'a mut [ParseStructOption<'a>],

    /// A boolean flag for each element of the `slots` slice that specifies
    /// whether the option(s) associated with the slot is a global option.
    pub slots_global: &'static [bool],

    /// A reference to the [`Help`] struct in the associated [`FromArgs`]. This
    /// is used to collect global options for generating a help message.
    pub help: &'static HelpInfo,

    /// If this struct represents options of a subcommand, then `parent` is an
    /// indirect reference to the previous [`ParseStructOptions`] in the chain,
    /// used for parsing global options.
    pub parent: Option<&'p mut dyn ParseGlobalOptions>,
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
    fn parse(&mut self, arg: &str, remaining_args: &mut &[&OsStr]) -> Result<(), Error> {
        match self.arg_to_slot.iter().find(|(name, _)| *name == arg) {
            Some((_, pos)) => Self::fill_slot(&mut self.slots[*pos], arg, remaining_args),
            None => self
                .try_parse_global(arg, remaining_args)
                .unwrap_or_else(|| Err(Error::UnknownArgument(OsString::from(arg)))),
        }
    }

    fn fill_slot(
        slot: &mut ParseStructOption<'a>,
        arg: &str,
        remaining_args: &mut &[&OsStr],
    ) -> Result<(), Error> {
        match slot {
            ParseStructOption::Flag(ref mut b) => b.set_flag(arg),
            ParseStructOption::Value(ref mut pvs) => {
                let value = remaining_args
                    .first()
                    .ok_or_else(|| Error::MissingArgValue(arg.to_owned()))?;
                *remaining_args = &remaining_args[1..];
                pvs.fill_slot(arg, value)?;
            }
        }
        Ok(())
    }
}

#[doc(hidden)]
pub trait ParseGlobalOptions {
    /// Parses a global command-line option. If the option is not found in
    /// _self_, it recursively calls this function on the parent. If the option
    /// is still not found, it returns `None`.
    ///
    /// - `arg`: The current option argument being parsed (e.g. `--foo`).
    /// - `remaining_args`: The remaining command line arguments. This slice
    ///    will be advanced forwards if the option takes a value argument.
    fn try_parse_global(
        &mut self,
        arg: &str,
        remaining_args: &mut &[&OsStr],
    ) -> Option<Result<(), Error>>;

    /// Returns a vector representing global options specified on this instance
    /// and recursively on the parent. This is used for generating a help
    /// message.
    fn global_options(&self) -> Vec<&'static OptionArgInfo>;
}

impl<'a, 'p> ParseGlobalOptions for ParseStructOptions<'a, 'p> {
    fn try_parse_global(
        &mut self,
        arg: &str,
        remaining_args: &mut &[&OsStr],
    ) -> Option<Result<(), Error>> {
        self.arg_to_slot
            .iter()
            .find(|(name, pos)| *name == arg && self.slots_global[*pos])
            .map(|(_, pos)| Self::fill_slot(&mut self.slots[*pos], arg, remaining_args))
            .or_else(|| {
                self.parent
                    .as_mut()
                    .and_then(|p| p.try_parse_global(arg, remaining_args))
            })
    }

    fn global_options(&self) -> Vec<&'static OptionArgInfo> {
        let mut opts = self
            .parent
            .as_ref()
            .map_or_else(Vec::new, |p| p.global_options());
        opts.extend(self.help.options.iter().filter(|o| o.global));
        opts
    }
}

/// `--` or `-` options, including a mutable reference to their value.
#[doc(hidden)]
pub enum ParseStructOption<'a> {
    /// A flag which is set to `true` when provided.
    Flag(&'a mut dyn ParseFlag),
    /// A value which is parsed from the string following the `--` argument,
    /// e.g. `--foo bar`.
    Value(&'a mut dyn ParseValueSlot),
}

#[doc(hidden)]
pub struct ParseStructPositionals<'a> {
    pub positionals: &'a mut [ParseStructPositional<'a>],
    pub last_is_repeating: bool,
    pub last_is_greedy: bool,
}

impl<'a> ParseStructPositionals<'a> {
    /// Parses the next positional argument.
    ///
    /// - `index`: The index of the argument.
    /// - `arg`: The argument supplied by the user.
    ///
    /// Returns `true` if non-positional argument parsing should stop after this
    /// one.
    fn parse(&mut self, index: &mut usize, arg: &OsStr) -> Result<bool, Error> {
        if *index < self.positionals.len() {
            self.positionals[*index].parse(arg)?;

            if self.last_is_repeating && *index == self.positionals.len() - 1 {
                // Don't increment position if we're at the last arg *and* the
                // last arg is repeating. If it's also remainder, halt
                // non-option processing after this.
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
    /// The positional's name
    pub name: &'static str,

    /// The function to parse the positional.
    pub slot: &'a mut dyn ParseValueSlot,
}

impl<'a> ParseStructPositional<'a> {
    /// Parses a positional argument.
    ///
    /// - `arg`: The argument supplied by the user.
    fn parse(&mut self, arg: &OsStr) -> Result<(), Error> {
        self.slot.fill_slot(self.name, arg)
    }
}

/// A type to simplify parsing struct subcommands.
///
/// This indirection is necessary to allow abstracting over [`FromArgs`]
/// instances with different generic parameters.
#[doc(hidden)]
pub struct ParseStructSubCommand<'a> {
    /// The subcommand commands.
    pub subcommands: &'static [&'static CommandInfo],

    pub dynamic_subcommands: &'a [&'static CommandInfo],

    /// The function to parse the subcommand arguments.
    #[allow(clippy::type_complexity)]
    pub parse_func: &'a mut dyn FnMut(
        &[&str],
        &[&OsStr],
        Option<&mut dyn ParseGlobalOptions>,
    ) -> Result<(), EarlyExit>,
}

impl<'a> ParseStructSubCommand<'a> {
    fn parse(
        &mut self,
        help: bool,
        cmd_name: &[&str],
        arg: &str,
        remaining_args: &[&OsStr],
        parse_global_opts: &mut dyn ParseGlobalOptions,
    ) -> Result<bool, EarlyExit> {
        for subcommand in self
            .subcommands
            .iter()
            .chain(self.dynamic_subcommands.iter())
        {
            if subcommand.name == arg {
                let mut command = cmd_name.to_owned();
                command.push(subcommand.name);

                let prepended_help;
                let remaining_args = if help {
                    prepended_help = [&[OsStr::new("help")], remaining_args].concat();
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

#[doc(hidden)]
pub trait ParseFlag {
    fn set_flag(&mut self, arg: &str);
}

impl<T: Flag> ParseFlag for T {
    fn set_flag(&mut self, _arg: &str) {
        <T as Flag>::set_flag(self);
    }
}

/// A trait for for slots that reserve space for a value and know how to parse
/// that value from a command-line `&OsStr` argument.
///
/// This trait is only implemented for the type [`ParseValueSlotTy`]. This
/// indirection is necessary to allow abstracting over [`ParseValueSlotTy`]
/// instances with different generic parameters.
#[doc(hidden)]
pub trait ParseValueSlot {
    fn fill_slot(&mut self, arg: &str, value: &OsStr) -> Result<(), Error>;
}

/// The concrete type implementing the [`ParseValueSlot`] trait.
///
/// - `T` is the type to be parsed from a single string.
/// - `Slot` is the type of the container that can hold a value or values of
///   type `T`.
#[doc(hidden)]
pub struct ParseValueSlotTy<Slot, T> {
    /// The slot for a parsed value.
    pub slot: Slot,
    /// The function to parse the value from a string
    pub parse_func: fn(&str, &OsStr) -> Result<T, String>,
}

/// `ParseValueSlotTy<Option<T>, T>` is used as the slot for all non-repeating
/// arguments, both optional and required.
impl<T> ParseValueSlot for ParseValueSlotTy<Option<T>, T> {
    fn fill_slot(&mut self, arg: &str, value: &OsStr) -> Result<(), Error> {
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

/// `ParseValueSlotTy<Vec<T>, T>` is used as the slot for repeating arguments.
impl<T> ParseValueSlot for ParseValueSlotTy<Vec<T>, T> {
    fn fill_slot(&mut self, arg: &str, value: &OsStr) -> Result<(), Error> {
        let parsed = (self.parse_func)(arg, value).map_err(|e| Error::ParseArgument {
            arg: arg.to_owned(),
            value: value.to_owned(),
            msg: e,
        })?;
        self.slot.push(parsed);
        Ok(())
    }
}

/// A type which can be the receiver of a [`Flag`].
#[doc(hidden)]
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
