/// Command
#[derive(argp::FromArgs)]
struct Cmd {
    #[argp(switch)]
    /// non-ascii
    привет: bool,
    #[argp(switch)]
    /// uppercase
    XMLHTTPRequest: bool,
    #[argp(switch, long = "not really")]
    /// bad attr
    ok: bool,
}

fn main() {}
