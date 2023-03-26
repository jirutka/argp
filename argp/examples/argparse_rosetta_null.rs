// This example is based on null-app example in argparse-rosetta-rs and it's
// used for measuring the size overhead.

use std::env;

fn main() {
    let args: Vec<_> = env::args_os().collect();

    if args.len() >= 10 {
        println!("{:#?}", args.len());
    } else {
        println!("{:#?}", args);
    }
}
