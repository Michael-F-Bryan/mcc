use compile_test::cmd::{self, Args};
use structopt::StructOpt;

fn main() -> Result<(), String> {
    let args = Args::from_args();
    cmd::run(&args)
}
