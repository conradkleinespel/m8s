use log::error;
use std::{env, process};

fn main() {
    let args = env::args().collect::<Vec<String>>();
    let args_refs = args.iter().map(|s| s.as_str()).collect::<Vec<&str>>();

    if let Err(err) = libm8scmd::main_with_args(args_refs) {
        error!("{}", err);
        process::exit(1);
    }
}
