// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2023 Jakub Jirutka <jakub@jirutka.cz>
// SPDX-FileCopyrightText: 2022 Google LLC

use std::fmt::Debug;

use argp::FromArgs;

/// Top-level command.
#[derive(FromArgs, PartialEq, Debug)]
struct TopLevel {
    /// Be verbose.
    #[argp(switch, global)]
    verbose: bool,

    #[argp(subcommand)]
    nested: MySubCommandEnum,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argp(subcommand)]
enum MySubCommandEnum {
    One(SubCommandOne),
    Two(SubCommandTwo),
}

/// First subcommand.
#[derive(FromArgs, PartialEq, Debug)]
#[argp(subcommand, name = "one")]
struct SubCommandOne {
    /// How many x.
    #[argp(option)]
    x: usize,
}

/// Second subcommand.
#[derive(FromArgs, PartialEq, Debug)]
#[argp(subcommand, name = "two")]
struct SubCommandTwo {
    /// Whether to fooey.
    #[argp(switch)]
    fooey: bool,
}

fn main() {
    let toplevel: TopLevel = argp::parse_args_or_exit();
    println!("{:#?}", toplevel);
}
