// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2022 Google LLC

use std::fmt::Debug;

use argp::FromArgs;

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

fn main() {
    let toplevel: TopLevel = argp::from_env();
    println!("{:#?}", toplevel);
}
