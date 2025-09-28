use anyhow::Context;
use integration_tests::ExpectedResults;
use libtest_mimic::Arguments;
use std::path::Path;

const MAX_CHAPTER: u32 = 3;
const EXPECTED_RESULTS: &str = include_str!("../writing-a-c-compiler-tests/expected_results.json");

fn main() -> anyhow::Result<()> {
    let args = Arguments::from_args();

    let test_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("writing-a-c-compiler-tests");

    let ignored = [
        // The following tests are expected to be parse errors, but we error
        // out when doing type checking instead.
        // The test suite expects `return int;` to be a parse error, but we
        // error out later on when doing type checking.
        "chapter_1::invalid_parse::not_expression", // return int;
        "chapter_3::invalid_parse::malformed_paren", // return 2 (- 3);
    ];
    let mut trials = Vec::new();
    let expected_results: ExpectedResults = serde_json::from_str(EXPECTED_RESULTS)?;

    for test in integration_tests::discover(&test_root, &expected_results)
        .context("failed to discover tests")?
    {
        let ignored = test.chapter > MAX_CHAPTER || ignored.contains(&test.name.as_str());
        let trial = test.trial().with_ignored_flag(ignored);
        trials.push(trial);
    }

    libtest_mimic::run(&args, trials).exit()
}
