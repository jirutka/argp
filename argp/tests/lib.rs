// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2023 Jakub Jirutka <jakub@jirutka.cz>
// SPDX-FileCopyrightText: 2020 Google LLC

// Deny a bunch of uncommon clippy lints to make sure the generated code won't trigger a warning.
#![deny(
    clippy::indexing_slicing,
    clippy::panic_in_result_fn,
    clippy::str_to_string,
    clippy::unreachable,
    clippy::unwrap_in_result
)]

use std::ffi::OsStr;
use std::fmt::Debug;

use argp::{
    CommandInfo, DynamicSubCommand, EarlyExit, Error, FromArgs, HelpStyle, MissingRequirements,
};

const EMPTY_ARGS: &[&OsStr] = &[];

const FIXED_HELP_STYLE: HelpStyle = HelpStyle {
    wrap_width_range: 80..80,
    ..HelpStyle::default()
};

#[test]
fn basic_example() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Reach new heights.
    struct GoUp {
        /// whether or not to jump
        #[argp(switch, short = 'j')]
        jump: bool,

        /// how high to go
        #[argp(option)]
        height: usize,

        /// an optional nickname for the pilot
        #[argp(option)]
        pilot_nickname: Option<String>,
    }

    let up = GoUp::from_args(&["cmdname"], &[OsStr::new("--height"), OsStr::new("5")])
        .expect("failed go_up");
    assert_eq!(
        up,
        GoUp {
            jump: false,
            height: 5,
            pilot_nickname: None
        }
    );
}

#[test]
fn generic_example() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Reach new heights.
    struct GoUp<S: argp::FromArgValue> {
        /// whether or not to jump
        #[argp(switch, short = 'j')]
        jump: bool,

        /// how high to go
        #[argp(option)]
        height: usize,

        /// an optional nickname for the pilot
        #[argp(option)]
        pilot_nickname: Option<S>,
    }

    let up = GoUp::<String>::from_args(&["cmdname"], &[OsStr::new("--height"), OsStr::new("5")])
        .expect("failed go_up");
    assert_eq!(
        up,
        GoUp::<String> {
            jump: false,
            height: 5,
            pilot_nickname: None
        }
    );
}

#[test]
fn custom_from_str_example() {
    mod submod {
        pub fn capitalize(value: &str) -> Result<String, String> {
            Ok(value.to_uppercase())
        }
    }

    #[derive(FromArgs)]
    /// Goofy thing.
    struct FiveStruct {
        /// always five
        #[argp(option, from_str_fn(always_five))]
        five: usize,

        #[argp(positional, from_str_fn(submod::capitalize))]
        msg: String,
    }

    fn always_five(_value: &str) -> Result<usize, String> {
        Ok(5)
    }

    let s =
        FiveStruct::from_args(&["cmdname"], &["--five", "woot", "hello"]).expect("failed to five");
    assert_eq!(s.five, 5);
    assert_eq!(s.msg, "HELLO");
}

#[test]
fn custom_from_os_str_example() {
    use std::path::PathBuf;

    #[derive(FromArgs)]
    /// Goofy thing.
    struct PathStruct {
        /// file path
        #[argp(option, from_os_str_fn(convert_path))]
        path: PathBuf,
    }

    fn convert_path(value: &OsStr) -> Result<PathBuf, String> {
        Ok(PathBuf::from(value))
    }

    let s = PathStruct::from_args(&["cmdname"], &[&OsStr::new("--path"), &OsStr::new("/foo/bar")])
        .expect("failed to parse");
    assert_eq!(s.path, PathBuf::from("/foo/bar"));
}

#[test]
fn subcommand_example() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Top-level command.
    struct TopLevel {
        #[argp(subcommand)]
        nested: MySubCommandEnum,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argp(subcommand)]
    enum MySubCommandEnum {
        One(SubCommandOne),
        Two(SubCommandTwo),
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// First subcommand.
    #[argp(subcommand, name = "one")]
    struct SubCommandOne {
        #[argp(option)]
        /// how many x
        x: usize,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// Second subcommand.
    #[argp(subcommand, name = "two")]
    struct SubCommandTwo {
        #[argp(switch)]
        /// whether to fooey
        fooey: bool,
    }

    let one = TopLevel::from_args(&["cmdname"], &["one", "--x", "2"]).expect("sc 1");
    assert_eq!(
        one,
        TopLevel {
            nested: MySubCommandEnum::One(SubCommandOne { x: 2 })
        },
    );

    let two = TopLevel::from_args(&["cmdname"], &["two", "--fooey"]).expect("sc 2");
    assert_eq!(
        two,
        TopLevel {
            nested: MySubCommandEnum::Two(SubCommandTwo { fooey: true })
        },
    );
}

#[test]
fn dynamic_subcommand_example() {
    #[derive(PartialEq, Debug)]
    struct DynamicSubCommandImpl {
        got: String,
    }

    impl DynamicSubCommand for DynamicSubCommandImpl {
        fn commands() -> &'static [&'static CommandInfo] {
            &[
                &CommandInfo {
                    name: "three",
                    description: "Third command",
                },
                &CommandInfo {
                    name: "four",
                    description: "Fourth command",
                },
                &CommandInfo {
                    name: "five",
                    description: "Fifth command",
                },
            ]
        }

        fn try_from_args(
            command_name: &[&str],
            args: &[&OsStr],
        ) -> Option<Result<DynamicSubCommandImpl, EarlyExit>> {
            let command_name = match command_name.last() {
                Some(x) => *x,
                None => return Some(Err(EarlyExit::Err(Error::other("No command")))),
            };
            let description = Self::commands()
                .iter()
                .find(|x| x.name == command_name)?
                .description;
            if args.len() > 1 {
                Some(Err(EarlyExit::Err(Error::other("Too many arguments"))))
            } else if let Some(arg) = args.first() {
                Some(Ok(DynamicSubCommandImpl {
                    got: format!("{} got {:?}", description, arg),
                }))
            } else {
                Some(Err(EarlyExit::Err(Error::other("Not enough arguments"))))
            }
        }
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// Top-level command.
    struct TopLevel {
        #[argp(subcommand)]
        nested: MySubCommandEnum,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argp(subcommand)]
    enum MySubCommandEnum {
        One(SubCommandOne),
        Two(SubCommandTwo),
        #[argp(dynamic)]
        ThreeFourFive(DynamicSubCommandImpl),
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// First subcommand.
    #[argp(subcommand, name = "one")]
    struct SubCommandOne {
        #[argp(option)]
        /// how many x
        x: usize,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// Second subcommand.
    #[argp(subcommand, name = "two")]
    struct SubCommandTwo {
        #[argp(switch)]
        /// whether to fooey
        fooey: bool,
    }

    let one = TopLevel::from_args(&["cmdname"], &["one", "--x", "2"]).expect("sc 1");
    assert_eq!(
        one,
        TopLevel {
            nested: MySubCommandEnum::One(SubCommandOne { x: 2 })
        },
    );

    let two = TopLevel::from_args(&["cmdname"], &["two", "--fooey"]).expect("sc 2");
    assert_eq!(
        two,
        TopLevel {
            nested: MySubCommandEnum::Two(SubCommandTwo { fooey: true })
        },
    );

    let three = TopLevel::from_args(&["cmdname"], &["three", "beans"]).expect("sc 3");
    assert_eq!(
        three,
        TopLevel {
            nested: MySubCommandEnum::ThreeFourFive(DynamicSubCommandImpl {
                got: "Third command got \"beans\"".to_owned()
            })
        },
    );

    let four = TopLevel::from_args(&["cmdname"], &["four", "boulders"]).expect("sc 4");
    assert_eq!(
        four,
        TopLevel {
            nested: MySubCommandEnum::ThreeFourFive(DynamicSubCommandImpl {
                got: "Fourth command got \"boulders\"".to_owned()
            })
        },
    );

    let five = TopLevel::from_args(&["cmdname"], &["five", "gold rings"]).expect("sc 5");
    assert_eq!(
        five,
        TopLevel {
            nested: MySubCommandEnum::ThreeFourFive(DynamicSubCommandImpl {
                got: "Fifth command got \"gold rings\"".to_owned()
            })
        },
    );
}

#[test]
fn multiline_doc_comment_description() {
    #[derive(FromArgs)]
    /// Short description
    struct Cmd {
        #[argp(switch)]
        /// a switch with a description
        /// that is spread across
        /// a number of
        /// lines of comments.
        _s: bool,
    }

    assert_help_string::<Cmd>(
        r###"Usage: test_arg_0 [--s]

Short description

Options:
      --s     a switch with a description that is spread across a number of
              lines of comments.
  -h, --help  Show this help message and exit.
"###,
    );
}

#[test]
fn default_number() {
    #[derive(FromArgs)]
    /// Short description
    struct Cmd {
        #[argp(option, default = "5")]
        /// fooey
        x: u8,
    }

    let cmd = Cmd::from_args(&["cmdname"], EMPTY_ARGS).unwrap();
    assert_eq!(cmd.x, 5);
}

#[test]
fn default_function() {
    const MSG: &str = "hey I just met you";
    fn call_me_maybe() -> String {
        MSG.to_owned()
    }

    #[derive(FromArgs)]
    /// Short description
    struct Cmd {
        #[argp(option, default = "call_me_maybe()")]
        /// fooey
        msg: String,
    }

    let cmd = Cmd::from_args(&["cmdname"], EMPTY_ARGS).unwrap();
    assert_eq!(cmd.msg, MSG);
}

#[test]
fn missing_option_value() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argp(option)]
        /// fooey
        _msg: String,
    }

    let e = Cmd::from_args(&["cmdname"], &["--msg"])
        .expect_err("Parsing missing option value should fail");
    assert_eq!(e, EarlyExit::Err(Error::MissingArgValue("--msg".to_owned())));
}

fn assert_help_string<T: FromArgs>(help_str: &str) {
    match T::from_args(&["test_arg_0"], &["--help"]) {
        Ok(_) => panic!("help was parsed as args"),
        Err(EarlyExit::Err(_)) => panic!("expected EarlyExit::Help"),
        Err(EarlyExit::Help(help)) => {
            assert_eq!(help.generate(&FIXED_HELP_STYLE), help_str);
        }
    }
}

fn assert_output<T: FromArgs + Debug + PartialEq>(args: &[&str], expected: T) {
    let t = T::from_args(&["cmd"], args).expect("failed to parse");
    assert_eq!(t, expected);
}

fn assert_error<T: FromArgs + Debug>(args: &[&str], expected: Error) {
    let e = T::from_args(&["cmd"], args).expect_err("unexpectedly succeeded parsing");
    assert_eq!(EarlyExit::Err(expected), e);
}

fn missing_requirements(
    positionals: &[&'static str],
    options: &[&'static str],
    subcommands: &[&'static str],
) -> MissingRequirements {
    let mut missing = MissingRequirements::default();

    for pos in positionals {
        missing.missing_positional_arg(pos);
    }
    for opt in options {
        missing.missing_option(opt);
    }
    if !subcommands.is_empty() {
        missing.missing_subcommands(subcommands.iter().copied());
    }

    missing
}

mod options {
    use super::*;

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct Parsed {
        #[argp(option, short = 'n')]
        /// fooey
        n: usize,
    }

    #[test]
    fn parsed() {
        assert_output(&["-n", "5"], Parsed { n: 5 });
        assert_error::<Parsed>(
            &["-n", "x"],
            Error::ParseArgument {
                arg: "-n".to_owned(),
                value: "x".into(),
                msg: "invalid digit found in string".to_owned(),
            },
        );
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct Repeating {
        #[argp(option, short = 'n')]
        /// fooey
        n: Vec<String>,
    }

    #[test]
    fn repeating() {
        assert_help_string::<Repeating>(
            r###"Usage: test_arg_0 [-n <n...>]

Woot

Options:
  -n, --n <n>  fooey
  -h, --help   Show this help message and exit.
"###,
        );
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct WithArgName {
        #[argp(option, arg_name = "name")]
        /// fooey
        option_name: Option<String>,
    }

    #[test]
    fn with_arg_name() {
        assert_help_string::<WithArgName>(
            r###"Usage: test_arg_0 [--option-name <name>]

Woot

Options:
      --option-name <name>  fooey
  -h, --help                Show this help message and exit.
"###,
        );
    }

    /// Woot
    #[derive(FromArgs, Debug, PartialEq)]
    struct ShortCombined {
        /// fooey
        #[argp(option, short = 'n')]
        n: usize,
        /// quiet
        #[argp(switch, short = 'q')]
        q: bool,
        /// verbose
        #[argp(switch, short = 'v')]
        v: bool,
    }

    #[test]
    fn short_combined() {
        assert_output(
            &["-qv", "-n", "5"],
            ShortCombined {
                n: 5,
                q: true,
                v: true,
            },
        );
        assert_output(
            &["-qvn", "5"],
            ShortCombined {
                n: 5,
                q: true,
                v: true,
            },
        );
        assert_error::<ShortCombined>(&["-nq", "5"], Error::MissingArgValue("-n".to_owned()));
    }
}

mod global_options {
    use super::*;

    #[derive(FromArgs, PartialEq, Debug)]
    /// Top level.
    struct TopLevel {
        #[argp(option, global, default = "0")]
        /// A global option a.
        a: usize,

        #[argp(option, default = "0")]
        /// A local option x.
        x: usize,

        #[argp(subcommand)]
        nested: FirstSubCommandEnum,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argp(subcommand)]
    enum FirstSubCommandEnum {
        One(SubCommandOne),
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// First subcommand.
    #[argp(subcommand, name = "one")]
    struct SubCommandOne {
        #[argp(switch, global)]
        /// A global option b.
        b: bool,

        #[argp(subcommand)]
        nested: Option<SecondSubCommandEnum>,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argp(subcommand)]
    enum SecondSubCommandEnum {
        Two(SubCommandTwo),
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// Second subcommand.
    #[argp(subcommand, name = "two")]
    struct SubCommandTwo {
        #[argp(switch)]
        /// Whether to fooey.
        fooey: bool,
    }

    fn expect_help(args: &[&str], expected_help_string: &str) {
        let exit_early = TopLevel::from_args(&["cmdname"], args).expect_err("should exit early");

        match exit_early {
            EarlyExit::Help(help) => {
                assert_eq!(expected_help_string, help.generate(&FIXED_HELP_STYLE))
            }
            _ => panic!("expected EarlyExit::Help"),
        }
    }

    #[test]
    fn parse() {
        for args in [&["--a", "1", "one", "--b"], &["one", "--a", "1", "--b"]] {
            let actual = TopLevel::from_args(&["cmdname"], args).expect("sc 1");
            assert_eq!(
                actual,
                TopLevel {
                    a: 1,
                    x: 0,
                    nested: FirstSubCommandEnum::One(SubCommandOne {
                        b: true,
                        nested: None
                    })
                },
            );
        }

        for args in [
            &["--a", "2", "one", "--b", "two"],
            &["one", "two", "--a", "2", "--b"],
        ] {
            let two = TopLevel::from_args(&["cmdname"], args).expect("sc 2");
            assert_eq!(
                two,
                TopLevel {
                    a: 2,
                    x: 0,
                    nested: FirstSubCommandEnum::One(SubCommandOne {
                        b: true,
                        nested: Some(SecondSubCommandEnum::Two(SubCommandTwo { fooey: false }))
                    })
                },
            );
        }
    }

    #[test]
    fn help() {
        expect_help(
            &["--help"],
            r###"Usage: cmdname [--a <a>] [--x <x>] <command> [<args>]

Top level.

Options:
      --a <a>  A global option a.
      --x <x>  A local option x.
  -h, --help   Show this help message and exit.

Commands:
  one          First subcommand.
"###,
        );

        expect_help(
            &["one", "--help"],
            r###"Usage: cmdname one [--a <a>] [--b] [<command>] [<args>]

First subcommand.

Options:
      --a <a>  A global option a.
      --b      A global option b.
  -h, --help   Show this help message and exit.

Commands:
  two          Second subcommand.
"###,
        );

        expect_help(
            &["one", "two", "--help"],
            r###"Usage: cmdname one two [--a <a>] [--b] [--fooey]

Second subcommand.

Options:
      --a <a>  A global option a.
      --b      A global option b.
      --fooey  Whether to fooey.
  -h, --help   Show this help message and exit.
"###,
        );
    }

    #[test]
    fn globals_are_not_propagated_up() {
        let e = TopLevel::from_args(&["cmdname"], &["one", "two", "--x", "6"])
            .expect_err("unexpectedly succeeded parsing sc 4");
        assert_eq!(e.to_string(), "Unrecognized argument: --x");
    }

    #[test]
    fn local_option_is_not_global() {
        let e = TopLevel::from_args(&["cmdname"], &["--b", "one"])
            .expect_err("unexpectedly succeeded parsing");
        assert_eq!(e.to_string(), "Unrecognized argument: --b");
    }
}

mod positional {
    use super::*;

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct LastRepeating {
        #[argp(positional)]
        /// fooey
        a: u32,
        #[argp(positional)]
        /// fooey
        b: Vec<String>,
    }

    #[test]
    fn repeating() {
        assert_output(&["5"], LastRepeating { a: 5, b: vec![] });
        assert_output(
            &["5", "foo"],
            LastRepeating {
                a: 5,
                b: vec!["foo".into()],
            },
        );
        assert_output(
            &["5", "foo", "bar"],
            LastRepeating {
                a: 5,
                b: vec!["foo".into(), "bar".into()],
            },
        );
        assert_help_string::<LastRepeating>(
            r###"Usage: test_arg_0 <a> [<b...>]

Woot

Arguments:
  a           fooey
  b           fooey

Options:
  -h, --help  Show this help message and exit.
"###,
        );
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct LastRepeatingGreedy {
        #[argp(positional)]
        /// fooey
        a: u32,
        #[argp(switch)]
        /// woo
        b: bool,
        #[argp(option)]
        /// stuff
        c: Option<String>,
        #[argp(positional, greedy)]
        /// fooey
        d: Vec<String>,
    }

    #[test]
    fn positional_greedy() {
        assert_output(
            &["5"],
            LastRepeatingGreedy {
                a: 5,
                b: false,
                c: None,
                d: vec![],
            },
        );
        assert_output(
            &["5", "foo"],
            LastRepeatingGreedy {
                a: 5,
                b: false,
                c: None,
                d: vec!["foo".into()],
            },
        );
        assert_output(
            &["5", "foo", "bar"],
            LastRepeatingGreedy {
                a: 5,
                b: false,
                c: None,
                d: vec!["foo".into(), "bar".into()],
            },
        );
        assert_output(
            &["5", "--b", "foo", "bar"],
            LastRepeatingGreedy {
                a: 5,
                b: true,
                c: None,
                d: vec!["foo".into(), "bar".into()],
            },
        );
        assert_output(
            &["5", "foo", "bar", "--b"],
            LastRepeatingGreedy {
                a: 5,
                b: false,
                c: None,
                d: vec!["foo".into(), "bar".into(), "--b".into()],
            },
        );
        assert_output(
            &["5", "--c", "hi", "foo", "bar"],
            LastRepeatingGreedy {
                a: 5,
                b: false,
                c: Some("hi".into()),
                d: vec!["foo".into(), "bar".into()],
            },
        );
        assert_output(
            &["5", "foo", "bar", "--c", "hi"],
            LastRepeatingGreedy {
                a: 5,
                b: false,
                c: None,
                d: vec!["foo".into(), "bar".into(), "--c".into(), "hi".into()],
            },
        );
        assert_output(
            &["5", "foo", "bar", "--", "hi"],
            LastRepeatingGreedy {
                a: 5,
                b: false,
                c: None,
                d: vec!["foo".into(), "bar".into(), "--".into(), "hi".into()],
            },
        );
        assert_help_string::<LastRepeatingGreedy>(
            r###"Usage: test_arg_0 [--b] [--c <c>] <a> [d...]

Woot

Arguments:
  a            fooey

Options:
      --b      woo
      --c <c>  stuff
  -h, --help   Show this help message and exit.
"###,
        );
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct LastOptional {
        #[argp(positional)]
        /// fooey
        a: u32,
        #[argp(positional)]
        /// fooey
        b: Option<String>,
    }

    #[test]
    fn optional() {
        assert_output(&["5"], LastOptional { a: 5, b: None });
        assert_output(
            &["5", "6"],
            LastOptional {
                a: 5,
                b: Some("6".into()),
            },
        );
        assert_error::<LastOptional>(&["5", "6", "7"], Error::UnknownArgument("7".into()));
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct LastDefaulted {
        #[argp(positional)]
        /// fooey
        a: u32,
        #[argp(positional, default = "5")]
        /// fooey
        b: u32,
    }

    #[test]
    fn defaulted() {
        assert_output(&["5"], LastDefaulted { a: 5, b: 5 });
        assert_output(&["5", "6"], LastDefaulted { a: 5, b: 6 });
        assert_error::<LastDefaulted>(&["5", "6", "7"], Error::UnknownArgument("7".into()));
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct LastRequired {
        #[argp(positional)]
        /// fooey
        a: u32,
        #[argp(positional)]
        /// fooey
        b: u32,
    }

    #[test]
    fn required() {
        assert_output(&["5", "6"], LastRequired { a: 5, b: 6 });
        assert_error::<LastRequired>(
            &[],
            Error::MissingRequirements(missing_requirements(&["a", "b"], &[], &[])),
        );
        assert_error::<LastRequired>(
            &["5"],
            Error::MissingRequirements(missing_requirements(&["b"], &[], &[])),
        );
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct Parsed {
        #[argp(positional)]
        /// fooey
        n: usize,
    }

    #[test]
    fn parsed() {
        assert_output(&["5"], Parsed { n: 5 });
        assert_error::<Parsed>(
            &["x"],
            Error::ParseArgument {
                arg: "n".to_owned(),
                value: "x".into(),
                msg: "invalid digit found in string".to_owned(),
            },
        );
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct WithOption {
        #[argp(positional)]
        /// fooey
        a: String,
        #[argp(option)]
        /// fooey
        b: String,
    }

    #[test]
    fn mixed_with_option() {
        assert_output(
            &["first", "--b", "foo"],
            WithOption {
                a: "first".into(),
                b: "foo".into(),
            },
        );

        assert_error::<WithOption>(
            &[],
            Error::MissingRequirements(missing_requirements(&["a"], &["--b"], &[])),
        );
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct WithSubcommand {
        #[argp(positional)]
        /// fooey
        a: String,
        #[argp(subcommand)]
        /// fooey
        b: Subcommand,
        #[argp(positional)]
        /// fooey
        c: Vec<String>,
    }

    #[derive(FromArgs, Debug, PartialEq)]
    #[argp(subcommand, name = "a")]
    /// Subcommand of positional::WithSubcommand.
    struct Subcommand {
        #[argp(positional)]
        /// fooey
        a: String,
        #[argp(positional)]
        /// fooey
        b: Vec<String>,
    }

    #[test]
    fn mixed_with_subcommand() {
        assert_output(
            &["first", "a", "a"],
            WithSubcommand {
                a: "first".into(),
                b: Subcommand {
                    a: "a".into(),
                    b: vec![],
                },
                c: vec![],
            },
        );

        assert_error::<WithSubcommand>(
            &["a", "a", "a"],
            Error::MissingRequirements(missing_requirements(&["a"], &[], &[])),
        );

        assert_output(
            &["1", "2", "3", "a", "b", "c"],
            WithSubcommand {
                a: "1".into(),
                b: Subcommand {
                    a: "b".into(),
                    b: vec!["c".into()],
                },
                c: vec!["2".into(), "3".into()],
            },
        );
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct Underscores {
        #[argp(positional)]
        /// fooey
        a_: String,
    }

    #[test]
    fn positional_name_with_underscores() {
        assert_output(&["first"], Underscores { a_: "first".into() });

        assert_error::<Underscores>(
            &[],
            Error::MissingRequirements(missing_requirements(&["a"], &[], &[])),
        );
    }

    #[derive(FromArgs)]
    /// Destroy the contents of <file>.
    struct WithArgName {
        #[argp(positional, arg_name = "name")]
        _username: String,
    }

    #[test]
    fn with_arg_name() {
        assert_help_string::<WithArgName>(
            r###"Usage: test_arg_0 <name>

Destroy the contents of <file>.

Arguments:
  name

Options:
  -h, --help  Show this help message and exit.
"###,
        );
    }

    /// Double-dash should be treated as the end of flags and optional arguments,
    /// and the remainder of the values should be treated purely as positional arguments,
    /// even when their syntax matches that of options. e.g. `foo -- -e` should be parsed
    /// as passing a single positional argument with the value `-e`.
    #[test]
    fn double_dash() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Positional arguments list
        struct StringList {
            #[argp(positional)]
            /// a list of strings
            strs: Vec<String>,

            #[argp(switch)]
            /// some flag
            flag: bool,
        }

        assert_output(
            &["--", "a", "-b", "--flag"],
            StringList {
                strs: vec!["a".into(), "-b".into(), "--flag".into()],
                flag: false,
            },
        );
        assert_output(
            &["--flag", "--", "-a", "b"],
            StringList {
                strs: vec!["-a".into(), "b".into()],
                flag: true,
            },
        );
        assert_output(
            &["--", "--help"],
            StringList {
                strs: vec!["--help".into()],
                flag: false,
            },
        );
        assert_output(
            &["--", "-a", "--help"],
            StringList {
                strs: vec!["-a".into(), "--help".into()],
                flag: false,
            },
        );
    }
}

/// Tests derived from
/// https://fuchsia.dev/fuchsia-src/development/api/cli and
/// https://fuchsia.dev/fuchsia-src/development/api/cli_help
mod fuchsia_commandline_tools_rubric {
    use super::*;

    #[derive(FromArgs, Debug)]
    /// One keyed option
    struct OneOption {
        #[argp(option)]
        /// some description
        _foo: String,
    }

    // When a tool has many subcommands, it should also have a help subcommand
    // that displays help about the subcommands, e.g. `fx help build`.
    //
    // Elsewhere in the docs, it says the syntax `--help` is required, so we
    // interpret that to mean:
    //
    // - `help` should always be accepted as a "keyword" in place of the first
    //   positional argument for both the main command and subcommands.
    //
    // - If followed by the name of a subcommand it should forward to the
    //   `--help` of said subcommand, otherwise it will fall back to the
    //   help of the righmost command / subcommand.
    //
    // - `--help` will always consider itself the only meaningful argument to
    //   the rightmost command / subcommand, and any following arguments will
    //   be treated as an error.

    #[derive(FromArgs, Debug)]
    /// A type for testing `--help`/`help`
    struct HelpTopLevel {
        #[argp(subcommand)]
        _sub: HelpFirstSub,
    }

    #[derive(FromArgs, Debug)]
    #[argp(subcommand, name = "first")]
    /// First subcommmand for testing `help`.
    struct HelpFirstSub {
        #[argp(subcommand)]
        _sub: HelpSecondSub,
    }

    #[derive(FromArgs, Debug)]
    #[argp(subcommand, name = "second")]
    /// Second subcommand for testing `help`.
    struct HelpSecondSub {}

    fn expect_help(args: &[&str], expected_help_string: &str) {
        let exit_early =
            HelpTopLevel::from_args(&["cmdname"], args).expect_err("should exit early");

        match exit_early {
            EarlyExit::Help(help) => {
                assert_eq!(expected_help_string, help.generate(&FIXED_HELP_STYLE))
            }
            _ => panic!("expected EarlyExit::Help"),
        }
    }

    const MAIN_HELP_STRING: &str = r###"Usage: cmdname <command> [<args>]

A type for testing `--help`/`help`

Options:
  -h, --help  Show this help message and exit.

Commands:
  first       First subcommmand for testing `help`.
"###;

    const FIRST_HELP_STRING: &str = r###"Usage: cmdname first <command> [<args>]

First subcommmand for testing `help`.

Options:
  -h, --help  Show this help message and exit.

Commands:
  second      Second subcommand for testing `help`.
"###;

    const SECOND_HELP_STRING: &str = r###"Usage: cmdname first second

Second subcommand for testing `help`.

Options:
  -h, --help  Show this help message and exit.
"###;

    #[test]
    fn help_keyword_main() {
        expect_help(&["help"], MAIN_HELP_STRING)
    }

    #[test]
    fn help_keyword_with_following_subcommand() {
        expect_help(&["help", "first"], FIRST_HELP_STRING);
    }

    #[test]
    fn help_keyword_between_subcommands() {
        expect_help(&["first", "help", "second"], SECOND_HELP_STRING);
    }

    #[test]
    fn help_keyword_with_two_trailing_subcommands() {
        expect_help(&["help", "first", "second"], SECOND_HELP_STRING);
    }

    #[test]
    fn help_flag_main() {
        expect_help(&["--help"], MAIN_HELP_STRING);
    }

    #[test]
    fn help_flag_subcommand() {
        expect_help(&["first", "--help"], FIRST_HELP_STRING);
    }

    // This was modified from testing the '--help' switch to the 'help'
    // subcommand.
    #[test]
    fn help_command_trailing_arguments_are_an_error() {
        let e = OneOption::from_args(&["cmdname"], &["help", "--foo", "bar"])
            .expect_err("should exit early");
        assert_eq!(EarlyExit::Err(Error::OptionsAfterHelp), e);
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argp(
        description = "Destroy the contents of <file>. Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation.\n\nDuis aute irure dolor in reprehenderit.",
        footer = "Examples:\n  Scribble 'abc' and then run |grind|.\n  $ test_arg_0 -s 'abc' grind old.txt taxes.cp",
        footer = "Notes:\n  Use `{command_name} help <command>` for details on [<args>] for a subcommand.",
        footer = "Error codes:\n  2 The blade is too dull.\n  3 Out of fuel."
    )]
    struct HelpExample {
        /// force, ignore minor errors. This description is so long that it wraps to the next line.
        #[argp(switch, short = 'f')]
        force: bool,

        /// documentation
        #[argp(switch)]
        really_really_really_long_name_for_pat: bool,

        /// write <scribble> repeatedly
        #[argp(option, short = 's')]
        scribble: String,

        /// say more. Defaults to $BLAST_VERBOSE.
        #[argp(switch, short = 'v')]
        verbose: bool,

        #[argp(subcommand)]
        command: HelpExampleSubCommands,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argp(subcommand)]
    enum HelpExampleSubCommands {
        BlowUp(BlowUp),
        Grind(GrindCommand),
        #[argp(dynamic)]
        Plugin(HelpExamplePlugin),
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argp(subcommand, name = "blow-up")]
    /// explosively separate
    struct BlowUp {
        /// blow up bombs safely
        #[argp(switch)]
        safely: bool,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argp(
        subcommand,
        name = "grind",
        description = "make smaller by many small cuts"
    )]
    struct GrindCommand {
        /// wear a visor while grinding
        #[argp(switch)]
        safely: bool,
    }

    #[derive(PartialEq, Debug)]
    struct HelpExamplePlugin {
        got: String,
    }

    impl DynamicSubCommand for HelpExamplePlugin {
        fn commands() -> &'static [&'static CommandInfo] {
            &[&CommandInfo {
                name: "plugin",
                description: "Example dynamic command",
            }]
        }

        fn try_from_args(
            command_name: &[&str],
            args: &[&OsStr],
        ) -> Option<Result<HelpExamplePlugin, EarlyExit>> {
            if command_name.last() != Some(&"plugin") {
                None
            } else if args.len() > 1 {
                Some(Err(EarlyExit::Err(Error::other("Too many arguments"))))
            } else if let Some(arg) = args.first() {
                Some(Ok(HelpExamplePlugin {
                    got: format!("plugin got {:?}", arg),
                }))
            } else {
                Some(Ok(HelpExamplePlugin {
                    got: "plugin got no argument".to_owned(),
                }))
            }
        }
    }

    #[test]
    fn example_parses_correctly() {
        let help_example = HelpExample::from_args(
            &["program-name"],
            &["-f", "--scribble", "fooey", "blow-up", "--safely"],
        )
        .unwrap();

        assert_eq!(
            help_example,
            HelpExample {
                force: true,
                scribble: "fooey".to_owned(),
                really_really_really_long_name_for_pat: false,
                verbose: false,
                command: HelpExampleSubCommands::BlowUp(BlowUp { safely: true }),
            },
        );
    }

    #[test]
    fn example_errors_on_missing_required_option_and_missing_required_subcommand() {
        let exit = HelpExample::from_args(&["program-name"], EMPTY_ARGS).unwrap_err();
        assert_eq!(
            exit,
            EarlyExit::Err(Error::MissingRequirements(missing_requirements(
                &[],
                &["--scribble"],
                &["blow-up", "grind", "plugin"]
            )))
        );
    }

    #[test]
    fn help_example() {
        assert_help_string::<HelpExample>(
            r###"Usage: test_arg_0 [-f] [--really-really-really-long-name-for-pat] -s <scribble>
                  [-v] <command> [<args>]

Destroy the contents of <file>. Lorem ipsum dolor sit amet, consectetur
adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna
aliqua. Ut enim ad minim veniam, quis nostrud exercitation.

Duis aute irure dolor in reprehenderit.

Options:
  -f, --force                force, ignore minor errors. This description is so
                             long that it wraps to the next line.
      --really-really-really-long-name-for-pat
                             documentation
  -s, --scribble <scribble>  write <scribble> repeatedly
  -v, --verbose              say more. Defaults to $BLAST_VERBOSE.
  -h, --help                 Show this help message and exit.

Commands:
  blow-up                    explosively separate
  grind                      make smaller by many small cuts
  plugin                     Example dynamic command

Examples:
  Scribble 'abc' and then run |grind|.
  $ test_arg_0 -s 'abc' grind old.txt taxes.cp

Notes:
  Use `test_arg_0 help <command>` for details on [<args>] for a subcommand.

Error codes:
  2 The blade is too dull.
  3 Out of fuel.
"###,
        );
    }

    #[test]
    fn hidden_help_attribute() {
        #[derive(FromArgs)]
        /// Short description
        struct Cmd {
            /// this one should be hidden
            #[argp(positional, hidden_help)]
            _one: String,
            #[argp(positional)]
            /// this one is real
            _two: String,
            /// this one should be hidden
            #[argp(option, hidden_help)]
            _three: String,
        }

        assert_help_string::<Cmd>(
            r###"Usage: test_arg_0 <two>

Short description

Arguments:
  two         this one is real

Options:
  -h, --help  Show this help message and exit.
"###,
        );
    }
}

mod parser {
    use super::*;

    #[test]
    fn no_args() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        struct Cmd {
            #[argp(option)]
            /// a msg param
            msg: Option<String>,
        }

        let actual = Cmd::from_args(&["program-name"], EMPTY_ARGS).unwrap();
        assert_eq!(actual, Cmd { msg: None });
    }

    #[test]
    fn optional_arg() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        struct Cmd {
            #[argp(option)]
            /// a msg param
            msg: Option<String>,
        }

        let actual = Cmd::from_args(&["program-name"], &["--msg", "hello"]).unwrap();
        assert_eq!(
            actual,
            Cmd {
                msg: Some("hello".to_owned())
            }
        );
    }

    #[test]
    fn optional_arg_short() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        struct Cmd {
            #[argp(option, short = 'm')]
            /// a msg param
            msg: Option<String>,
        }

        let actual = Cmd::from_args(&["program-name"], &["-m", "hello"]).unwrap();
        assert_eq!(
            actual,
            Cmd {
                msg: Some("hello".to_owned())
            }
        );
    }

    #[test]
    fn optional_arg_long() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        struct Cmd {
            #[argp(option, long = "my-msg")]
            /// a msg param
            msg: Option<String>,
        }

        let actual = Cmd::from_args(&["program-name"], &["--my-msg", "hello"]).unwrap();
        assert_eq!(
            actual,
            Cmd {
                msg: Some("hello".to_owned())
            }
        );
    }

    #[test]
    fn two_option_args() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        struct Cmd {
            #[argp(option)]
            /// a msg param
            msg: String,

            #[argp(option)]
            /// a delivery param
            delivery: String,
        }

        let actual =
            Cmd::from_args(&["program-name"], &["--msg", "hello", "--delivery", "next day"])
                .unwrap();
        assert_eq!(
            actual,
            Cmd {
                msg: "hello".to_owned(),
                delivery: "next day".to_owned(),
            }
        );
    }

    #[test]
    fn option_one_optional_args() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        struct Cmd {
            #[argp(option)]
            /// a msg param
            msg: String,

            #[argp(option)]
            /// a delivery param
            delivery: Option<String>,
        }

        let actual =
            Cmd::from_args(&["program-name"], &["--msg", "hello", "--delivery", "next day"])
                .unwrap();
        assert_eq!(
            actual,
            Cmd {
                msg: "hello".to_owned(),
                delivery: Some("next day".to_owned())
            },
        );

        let actual = Cmd::from_args(&["program-name"], &["--msg", "hello"]).unwrap();
        assert_eq!(
            actual,
            Cmd {
                msg: "hello".to_owned(),
                delivery: None
            },
        );
    }

    #[test]
    fn option_repeating() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        struct Cmd {
            #[argp(option)]
            /// fooey
            msg: Vec<String>,
        }

        let actual = Cmd::from_args(&["program-name"], EMPTY_ARGS).unwrap();
        assert_eq!(actual, Cmd { msg: vec![] });

        let actual = Cmd::from_args(&["program-name"], &["--msg", "abc", "--msg", "xyz"]).unwrap();
        assert_eq!(
            actual,
            Cmd {
                msg: vec!["abc".to_owned(), "xyz".to_owned()]
            }
        );
    }

    #[test]
    fn switch() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        struct Cmd {
            #[argp(switch, short = 'f')]
            /// speed of cmd
            faster: bool,
        }

        let actual = Cmd::from_args(&["program-name"], EMPTY_ARGS).unwrap();
        assert_eq!(actual, Cmd { faster: false });

        let actual = Cmd::from_args(&["program-name"], &["--faster"]).unwrap();
        assert_eq!(actual, Cmd { faster: true });

        let actual = Cmd::from_args(&["program-name"], &["-f"]).unwrap();
        assert_eq!(actual, Cmd { faster: true });
    }

    /// Repeating switches may be used to apply more emphasis.
    /// A common example is increasing verbosity by passing more `-v` switches.
    #[test]
    fn switch_repeating() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        struct Cmd {
            #[argp(switch, short = 'v')]
            /// increase the verbosity of the command.
            verbose: i128,
        }

        let actual = Cmd::from_args(&["cmdname"], &["-v", "-v", "-v"])
            .expect("Parsing verbose flags should succeed");
        assert_eq!(actual, Cmd { verbose: 3 });
    }

    #[test]
    fn positional() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        struct Cmd {
            #[allow(unused)]
            #[argp(positional)]
            /// speed of cmd
            speed: u8,
        }

        let actual = Cmd::from_args(&["program-name"], &["5"]).unwrap();
        assert_eq!(actual, Cmd { speed: 5 });
    }

    #[test]
    fn positional_arg_name() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        struct Cmd {
            #[argp(positional, arg_name = "speed")]
            /// speed of cmd
            speed: u8,
        }

        let actual = Cmd::from_args(&["program-name"], &["5"]).unwrap();
        assert_eq!(actual, Cmd { speed: 5 });
    }

    #[test]
    fn positional_repeating() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        struct Cmd {
            #[argp(positional, arg_name = "speed")]
            /// speed of cmd
            speed: Vec<u8>,
        }

        let actual = Cmd::from_args(&["program-name"], &["5", "6"]).unwrap();
        assert_eq!(actual, Cmd { speed: vec![5, 6] });
    }

    #[test]
    fn positional_err() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        struct Cmd {
            #[argp(positional, arg_name = "speed")]
            /// speed of cmd
            speed: u8,
        }

        let actual = Cmd::from_args(&["program-name"], EMPTY_ARGS).unwrap_err();
        assert_eq!(
            actual,
            EarlyExit::Err(Error::MissingRequirements(missing_requirements(&["speed"], &[], &[])))
        );
    }

    #[test]
    fn two_positional() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        struct Cmd {
            #[argp(positional, arg_name = "speed")]
            /// speed of cmd
            speed: u8,

            #[argp(positional, arg_name = "direction")]
            /// direction
            direction: String,
        }

        let actual = Cmd::from_args(&["program-name"], &["5", "north"]).unwrap();
        assert_eq!(
            actual,
            Cmd {
                speed: 5,
                direction: "north".to_owned()
            }
        );
    }

    #[test]
    fn positional_option() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        struct Cmd {
            #[argp(positional, arg_name = "speed")]
            /// speed of cmd
            speed: u8,

            #[argp(option)]
            /// direction
            direction: String,
        }

        let actual = Cmd::from_args(&["program-name"], &["5", "--direction", "north"]).unwrap();
        assert_eq!(
            actual,
            Cmd {
                speed: 5,
                direction: "north".to_owned()
            }
        );
    }

    #[test]
    fn positional_optional_option() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        struct Cmd {
            #[argp(positional, arg_name = "speed")]
            /// speed of cmd
            speed: u8,

            #[argp(option)]
            /// direction
            direction: Option<String>,
        }

        let actual = Cmd::from_args(&["program-name"], &["5"]).unwrap();
        assert_eq!(
            actual,
            Cmd {
                speed: 5,
                direction: None
            }
        );
    }

    #[test]
    fn subcommand() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        struct Cmd {
            #[argp(positional, arg_name = "speed")]
            /// speed of cmd
            speed: u8,

            #[argp(subcommand)]
            /// means of transportation
            means: MeansSubcommand,
        }

        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        #[argp(subcommand)]
        enum MeansSubcommand {
            Walking(WalkingSubcommand),
            Biking(BikingSubcommand),
            Driving(DrivingSubcommand),
        }

        #[derive(FromArgs, Debug, PartialEq)]
        #[argp(subcommand, name = "walking")]
        /// Short description
        struct WalkingSubcommand {
            #[argp(option)]
            /// a song to listen to
            music: String,
        }

        #[derive(FromArgs, Debug, PartialEq)]
        #[argp(subcommand, name = "biking")]
        /// Short description
        struct BikingSubcommand {}
        #[derive(FromArgs, Debug, PartialEq)]
        #[argp(subcommand, name = "driving")]
        /// short description
        struct DrivingSubcommand {}

        let actual =
            Cmd::from_args(&["program-name"], &["5", "walking", "--music", "Bach"]).unwrap();
        assert_eq!(
            actual,
            Cmd {
                speed: 5,
                means: MeansSubcommand::Walking(WalkingSubcommand {
                    music: "Bach".to_owned()
                })
            }
        );
    }

    #[test]
    fn subcommand_with_space_in_name() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        struct Cmd {
            #[argp(positional, arg_name = "speed")]
            /// speed of cmd
            speed: u8,

            #[argp(subcommand)]
            /// means of transportation
            means: MeansSubcommand,
        }

        #[derive(FromArgs, Debug, PartialEq)]
        /// Short description
        #[argp(subcommand)]
        enum MeansSubcommand {
            Walking(WalkingSubcommand),
            Biking(BikingSubcommand),
        }

        #[derive(FromArgs, Debug, PartialEq)]
        #[argp(subcommand, name = "has space")]
        /// Short description
        struct WalkingSubcommand {
            #[argp(option)]
            /// a song to listen to
            music: String,
        }

        #[derive(FromArgs, Debug, PartialEq)]
        #[argp(subcommand, name = "biking")]
        /// Short description
        struct BikingSubcommand {}

        let actual =
            Cmd::from_args(&["program-name"], &["5", "has space", "--music", "Bach"]).unwrap();
        assert_eq!(
            actual,
            Cmd {
                speed: 5,
                means: MeansSubcommand::Walking(WalkingSubcommand {
                    music: "Bach".to_owned()
                })
            }
        );
    }

    #[test]
    fn produces_help() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Woot
        struct Repeating {
            #[argp(option, short = 'n')]
            /// fooey
            n: Vec<String>,
        }

        let early_exit = Repeating::from_args(&["program-name"], &["--help"])
            .expect_err("unexpectedly succeeded parsing");

        if let EarlyExit::Help(help) = early_exit {
            assert_eq!(
                help.generate(&FIXED_HELP_STYLE),
                r###"Usage: program-name [-n <n...>]

Woot

Options:
  -n, --n <n>  fooey
  -h, --help   Show this help message and exit.
"###
            )
        } else {
            panic!("expected EarlyExit::Help");
        }
    }

    #[test]
    fn produces_errors_with_bad_arguments() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Woot
        struct Cmd {
            #[argp(option, short = 'n')]
            /// fooey
            n: String,
        }

        assert_eq!(
            Cmd::from_args(&["program-name"], &["--n"]),
            Err(EarlyExit::Err(Error::MissingArgValue("--n".to_owned()))),
        );
    }

    #[test]
    fn does_not_warn_if_used() {
        #[forbid(unused)]
        #[derive(FromArgs, Debug)]
        /// Short description
        struct Cmd {
            #[argp(positional)]
            /// speed of cmd
            speed: u8,
        }

        let cmd = Cmd::from_args(&["program-name"], &["5"]).unwrap();
        assert_eq!(cmd.speed, 5);
    }

    #[test]
    #[cfg(unix)]
    fn handles_args_with_invalid_utf8() {
        use std::ffi::OsString;
        use std::os::unix::prelude::OsStrExt;
        use std::path::PathBuf;

        #[derive(FromArgs)]
        /// Goofy thing.
        struct Cmd {
            /// message
            #[argp(option, short = 'm')]
            msg: OsString,

            /// path
            #[argp(positional)]
            path: PathBuf,
        }

        let msg = OsStr::from_bytes(&[b'f', b'o', 0x80, b'o']);
        let path = OsStr::from_bytes(&[b'/', b'f', b'o', 0x80, b'o']);

        let s =
            Cmd::from_args(&["cmdname"], &[OsStr::new("-m"), msg, path]).expect("failed to parse");
        assert_eq!(s.msg, msg.to_os_string());
        assert_eq!(s.path, PathBuf::from(path));
    }

    #[test]
    #[cfg(windows)]
    fn handles_args_with_invalid_utf8() {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use std::path::PathBuf;

        #[derive(FromArgs)]
        /// Goofy thing.
        struct Cmd {
            /// message
            #[argp(option, short = 'm')]
            msg: OsString,

            /// path
            #[argp(positional)]
            path: PathBuf,
        }

        let msg = OsString::from_wide(&[0x0066, 0x006F, 0xD800, 0x006F]);
        let path = OsString::from_wide(&[0x0066, 0x006F, 0xD800, 0x006F]);

        let s = Cmd::from_args(&["cmdname"], &[&OsString::from("-m"), &msg, &path])
            .expect("failed to parse");
        assert_eq!(s.msg, msg);
        assert_eq!(s.path, PathBuf::from(path));
    }
}

#[test]
fn subcommand_does_not_panic() {
    #[derive(FromArgs, PartialEq, Debug)]
    #[argp(subcommand)]
    enum SubCommandEnum {
        Cmd(SubCommand),
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// First subcommand.
    #[argp(subcommand, name = "one")]
    struct SubCommand {
        #[argp(positional)]
        /// how many x
        x: usize,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// Second subcommand.
    #[argp(subcommand, name = "two")]
    struct SubCommandTwo {
        #[argp(switch)]
        /// whether to fooey
        fooey: bool,
    }

    // Passing no subcommand name to an emum
    assert_eq!(
        SubCommandEnum::from_args(&[], &["5"]).unwrap_err(),
        EarlyExit::Err(Error::other("no subcommand name")),
    );

    // Passing unknown subcommand name to an emum
    assert_eq!(
        SubCommandEnum::from_args(&["fooey"], &["5"]).unwrap_err(),
        EarlyExit::Err(Error::other("no subcommand matched")),
    );
}

#[test]
fn long_alphanumeric() {
    #[derive(FromArgs)]
    /// Short description
    struct Cmd {
        #[argp(option, long = "ac97")]
        /// fooey
        ac97: String,
    }

    let cmd = Cmd::from_args(&["cmdname"], &["--ac97", "bar"]).unwrap();
    assert_eq!(cmd.ac97, "bar");
}
