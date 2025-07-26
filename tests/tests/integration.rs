use libtest_mimic::Arguments;
use std::path::Path;

const MAX_CHAPTER: u32 = 1;

fn main() -> anyhow::Result<()> {
    let args = Arguments::from_args();

    let test_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("writing-a-c-compiler-tests");

    let test_cases = tests::discover(&test_root)?;

    let trials = test_cases
        .into_iter()
        .map(|tc| tc.trial(MAX_CHAPTER))
        .collect();

    libtest_mimic::run(&args, trials).exit()
}
