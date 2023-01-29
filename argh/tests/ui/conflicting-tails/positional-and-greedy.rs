/// Command
#[derive(argp::FromArgs)]
struct Cmd {
    #[argp(positional)]
    /// positional
    positional: Vec<String>,

    #[argp(positional, greedy)]
    /// remainder
    remainder: Vec<String>,
}

fn main() {}
