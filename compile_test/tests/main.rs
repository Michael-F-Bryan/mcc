use compile_test::cmd::{self, Args};
use structopt::StructOpt;

use std::path::Path;

fn main() -> Result<(), String> {
    let args = Args {
        fixture_dir: Path::new(env!("CARGO_MANIFEST_DIR")).join("tests"),
        verbosity: 1,
    };

    cmd::run(&args)
}
