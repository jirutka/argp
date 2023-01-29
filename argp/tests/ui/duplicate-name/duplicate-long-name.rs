/// Command
#[derive(argp::FromArgs)]
struct Cmd {
    /// foo1
    #[argp(option, long = "foo")]
    foo1: u32,

    /// foo2
    #[argp(option, long = "foo")]
    foo2: u32,

    /// bar1
    #[argp(option, long = "bar")]
    bar1: u32,

    /// bar2
    #[argp(option, long = "bar")]
    bar2: u32,
}

fn main() {}
