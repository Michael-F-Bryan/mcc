use crate::parse_fail::ParseFail;
use crate::{Outcome, TestCase};
use failure::Error;
use slog::Logger;
use std::path::Path;

pub fn run<P: AsRef<Path>>(root: P, logger: &Logger) -> Result<(), Error> {
    let root = root.as_ref();

    let suite = load_test_suite(root, logger)?;
    let mut failures: Vec<&dyn TestCase> = Vec::new();

    if !suite.parse_fail.is_empty() {
        info!(logger, "Executing parse-fail tests");

        for test in &suite.parse_fail {
            debug!(logger, "Executing test"; "name" => test.name());

            match test.run() {
                Outcome::Pass => info!(logger, "Test Passed"; "test-name" => test.name()),
                Outcome::SetupFail(e) => {
                    error!(logger, "Test Setup Failed";
                          "error" => e.to_string(),
                          "test-name" => test.name());
                    debug!(logger, "Backtrace";
                           "bt" => &format_args!("{}", e.backtrace()));

                    failures.push(test);
                }
                Outcome::Fail(e) => {
                    warn!(logger, "Test Failed";
                          "error" => e.to_string(),
                          "test-name" => test.name());
                    debug!(logger, "Backtrace";
                           "bt" => &format_args!("{}", e.backtrace()));

                    failures.push(test);
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug, Default, Clone)]
pub struct TestSuite {
    pub parse_fail: Vec<ParseFail>,
}

fn load_test_suite(root: &Path, logger: &Logger) -> Result<TestSuite, Error> {
    info!(logger, "Loading test suite"; "root" => root.display());
    let mut suite = TestSuite::default();

    let parse_fail_dir = root.join("parse-fail");
    if parse_fail_dir.is_dir() {
        for entry in parse_fail_dir.read_dir()? {
            let entry = entry?;

            if entry.path().extension() == Some("c".as_ref()) {
                let test = ParseFail::for_file(entry.path())?;
                debug!(logger, "Found a parse-fail test";
                       "filename" => test.file_name().display(),
                       "name" => test.name());
                suite.parse_fail.push(test);
            }
        }
    }

    debug!(logger, "Test suite loaded";
           "parse-fail-test" => suite.parse_fail.len());
    Ok(suite)
}
