/// Command
#[derive(argp::FromArgs)]
struct Cmd {
    /// foo1
    #[argp(option, short = 'f')]
    foo1: u32,

    /// foo2
    #[argp(option, short = 'f')]
    foo2: u32,

    /// bar1
    #[argp(option, short = 'b')]
    bar1: u32,

    /// bar2
    #[argp(option, short = 'b')]
    bar2: u32,
}

fn main() {}
