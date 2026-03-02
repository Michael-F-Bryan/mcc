//! Integration test harness for the C compiler using the writing-a-c-compiler-tests suite.
//!
//! ## Test Discovery
//!
//! Tests are automatically discovered by scanning the `writing-a-c-compiler-tests/tests/` directory
//! structure:
//!
//! - The test suite is organized into chapters: `chapter_1/`, `chapter_2/`, etc.
//! - Each chapter contains subdirectories for test kinds:
//!   - `valid/` - Tests that should compile and run successfully
//!   - `invalid_parse/` - Tests that should fail during lexing or parsing
//!   - `invalid_tacky/` - Tests that should fail during lowering to TACKY IR
//!   - `invalid_codegen/` - Tests that should fail during code generation
//! - Within each kind directory, individual `.c` files are discovered as test cases
//! - Test names are formatted as: `chapter_{n}::{kind}::{filename}`
//!
//! Expected results (return codes and stdout) are loaded from `expected_results.json` in the
//! test suite directory. Each test verifies that compilation produces the expected behavior.
//!
//! ## Usage
//!
//! This test harness integrates with [libtest-mimic](https://docs.rs/libtest-mimic/) to provide
//! a standard Rust test runner interface. Tests are typically run with `cargo nextest`, though
//! `cargo test` also works:
//!
//! ```bash
//! # Run all tests up to chapter 4 (as configured by MAX_CHAPTER)
//! cargo nextest run --test integration
//!
//! # Run with specific filters
//! cargo nextest run --test integration -- chapter_1::valid::
//!
//! # Or using standard cargo test
//! cargo test --test integration
//! ```
//!
//! Tests can be selectively ignored by:
//! - Setting `MAX_CHAPTER` to limit which chapters are tested
//! - Adding test names to the `ignored` array **only** when our design structurally differs from
//!   the book (e.g. we report an error at type-check instead of parse). Do NOT use ignored for
//!   unimplemented features—leave those tests failing until the feature is implemented.

use anyhow::Context;
use integration_tests::ExpectedResults;
use libtest_mimic::Arguments;
use std::path::Path;

const MAX_CHAPTER: u32 = 6;
const EXPECTED_RESULTS: &str = include_str!("../writing-a-c-compiler-tests/expected_results.json");

fn main() -> anyhow::Result<()> {
    let args = Arguments::from_args();

    let test_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("writing-a-c-compiler-tests");

    let ignored = [
        // Structural difference: we report these during type checking, not parsing.
        "chapter_1::invalid_parse::not_expression", // return int;
        "chapter_1::invalid_parse::missing_type", // main(void){...}: we report missing return type at typecheck
        "chapter_3::invalid_parse::malformed_paren", // return 2 (- 3);
        "chapter_5::invalid_parse::invalid_type", // invalid type specifier: we report at typecheck
        "chapter_5::invalid_parse::declare_keyword_as_var", // keyword as identifier: we report at typecheck
        "chapter_5::invalid_semantics::invalid_lvalue", // a+3=4: we reject at parse (grammar), book expects semantics
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
