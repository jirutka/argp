# Argp
**Argp is a Derive-based argument parser optimized for code size**

[![crates.io](https://img.shields.io/crates/v/argp.svg)](https://crates.io/crates/argp)
[![license](https://img.shields.io/badge/license-BSD3.0-blue.svg)](https://github.com/jirutka/argp/LICENSE)
[![docs.rs](https://docs.rs/argp/badge.svg)](https://docs.rs/crate/argp/)
![CI](https://github.com/jirutka/argp/workflows/CI/badge.svg)

Derive-based argument parsing optimized for code size and flexibility.

The public API of this library consists primarily of the `FromArgs`
derive and the `from_env` function, which can be used to produce
a top-level `FromArgs` type from the current program's command-line
arguments.

Argp is originally a fork of [argh](https://github.com/google/argh/).

## Basic Example

```rust,no_run
use argp::FromArgs;

#[derive(FromArgs)]
/// Reach new heights.
struct GoUp {
    /// whether or not to jump
    #[argp(switch, short = 'j')]
    jump: bool,

    /// how high to go
    #[argp(option, arg_name = "meters")]
    height: usize,

    /// an optional nickname for the pilot
    #[argp(option, arg_name = "name")]
    pilot_nickname: Option<String>,
}

fn main() {
    let up: GoUp = argp::from_env();
}
```

`./some_bin --help` will then output the following:

```
Usage: cmdname [-j] --height <meters> [--pilot-nickname <name>]

Reach new heights.

Options:
  -j, --jump               whether or not to jump
  --height <meters>        how high to go
  --pilot-nickname <name>  an optional nickname for the pilot
  -h, --help               Show this help message and exit
```

The resulting program can then be used in any of these ways:
- `./some_bin --height 5`
- `./some_bin -j --height 5`
- `./some_bin --jump --height 5 --pilot-nickname Wes`

Switches, like `jump`, are optional and will be set to true if provided.

Options, like `height` and `pilot_nickname`, can be either required,
optional, or repeating, depending on whether they are contained in an
`Option` or a `Vec`. Default values can be provided using the
`#[argp(default = "<your_code_here>")]` attribute, and in this case an
option is treated as optional.

```rust
use argp::FromArgs;

fn default_height() -> usize {
    5
}

#[derive(FromArgs)]
/// Reach new heights.
struct GoUp {
    /// an optional nickname for the pilot
    #[argp(option)]
    pilot_nickname: Option<String>,

    /// an optional height
    #[argp(option, default = "default_height()")]
    height: usize,

    /// an optional direction which is "up" by default
    #[argp(option, default = "String::from(\"only up\")")]
    direction: String,
}

fn main() {
    let up: GoUp = argp::from_env();
}
```

Custom option types can be deserialized so long as they implement the
`FromArgValue` trait (automatically implemented for all `FromStr` types).
If more customized parsing is required, you can supply a custom
`fn(&str) -> Result<T, String>` using the `from_str_fn` attribute:

```rust
use argp::FromArgs;

#[derive(FromArgs)]
/// Goofy thing.
struct FiveStruct {
    /// always five
    #[argp(option, from_str_fn(always_five))]
    five: usize,
}

fn always_five(_value: &str) -> Result<usize, String> {
    Ok(5)
}
```

Positional arguments can be declared using `#[argp(positional)]`.
These arguments will be parsed in order of their declaration in
the structure:

```rust
use argp::FromArgs;

#[derive(FromArgs, PartialEq, Debug)]
/// A command with positional arguments.
struct WithPositional {
    #[argp(positional)]
    first: String,
}
```

The last positional argument may include a default, or be wrapped in
`Option` or `Vec` to indicate an optional or repeating positional argument.

Subcommands are also supported. To use a subcommand, declare a separate
`FromArgs` type for each subcommand as well as an enum that cases
over each command:

```rust
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
```

## How to debug the expanded derive macro for `argp`

The `argp::FromArgs` derive macro can be debugged with the [cargo-expand](https://crates.io/crates/cargo-expand) crate.

### Expand the derive macro in `examples/simple_example.rs`

See [argp/examples/simple_example.rs](./argp/examples/simple_example.rs) for the example struct we wish to expand.

First, install `cargo-expand` by running `cargo install cargo-expand`. Note this requires the nightly build of Rust.

Once installed, run `cargo expand` with in the `argp` package and you can see the expanded code.
