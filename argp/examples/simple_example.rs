// Copyright (c) 2022 Google LLC All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use {argp::FromArgs, std::fmt::Debug};

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
