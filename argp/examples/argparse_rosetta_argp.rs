// This example is based on argparse-rosetta-rs examples and it's used for
// measuring the size overhead.

use argp::FromArgs;

/// App
#[derive(Debug, FromArgs)]
struct AppArgs {
    /// sets number
    #[argp(option)]
    number: u32,

    /// sets optional number
    #[argp(option)]
    opt_number: Option<u32>,

    /// sets width [default: 10]
    #[argp(option, default = "10", from_str_fn(parse_width))]
    width: u32,

    /// input
    #[argp(positional)]
    input: Vec<std::path::PathBuf>,
}

fn parse_width(s: &str) -> Result<u32, String> {
    let w = s.parse().map_err(|_| "not a number")?;
    if w != 0 {
        Ok(w)
    } else {
        Err("width must be positive".to_string())
    }
}

fn main() {
    let args: AppArgs = argp::parse_args_or_exit(argp::DEFAULT);
    println!("{:#?}", args.number);
    println!("{:#?}", args.opt_number);
    println!("{:#?}", args.width);
    if args.input.len() >= 10 {
        println!("{:#?}", args.input.len());
    } else {
        println!("{:#?}", args);
    }
}
