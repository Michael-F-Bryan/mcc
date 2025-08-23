use anyhow::Context;
use libtest_mimic::Arguments;
use std::path::Path;
use tests::ExpectedResults;

const MAX_CHAPTER: u32 = 3;
const EXPECTED_RESULTS: &str = include_str!("../writing-a-c-compiler-tests/expected_results.json");

fn main() -> anyhow::Result<()> {
    let args = Arguments::from_args();

    let test_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("writing-a-c-compiler-tests");

    let ignored = ["chapter_1::invalid_parse::not_expression"];
    let mut trials = Vec::new();
    let expected_results: ExpectedResults = serde_json::from_str(EXPECTED_RESULTS)?;

    for test in
        tests::discover(&test_root, &expected_results).context("failed to discover tests")?
    {
        let ignored = test.chapter > MAX_CHAPTER || ignored.contains(&test.name.as_str());
        let trial = test.trial().with_ignored_flag(ignored);
        trials.push(trial);
    }

    libtest_mimic::run(&args, trials).exit()
}
