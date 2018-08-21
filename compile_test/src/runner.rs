use crate::parse_fail::ParseFail;
use crate::run_pass::RunPass;
use crate::{Outcome, TestCase};
use failure::Error;
use slog::Logger;
use std::any::Any;
use std::panic;
use std::path::Path;

pub fn run<P: AsRef<Path>>(root: P, logger: &Logger) -> Result<(), Error> {
    let root = root.as_ref();

    let suite = load_test_suite(root, logger)?;
    let mut failures: Vec<&dyn TestCase> = Vec::new();

    for test in suite.tests() {
        debug!(logger, "Executing test";
                   "name" => test.name(),
                   "category" => test.category());
        execute_test(test, &mut failures, &logger);
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failure::err_msg("One or more tests failed"))
    }
}

fn execute_test<'a>(test: &'a dyn TestCase, failures: &mut Vec<&'a dyn TestCase>, logger: &Logger) {
    let result = panic::catch_unwind(|| test.run());

    let outcome = match result {
        Ok(outcome) => outcome,
        Err(e) => interpret_panic_message(e),
    };

    match outcome {
        Outcome::Pass => info!(logger, "Test Passed"; "test-name" => test.name()),
        Outcome::SetupFail(e) => {
            error!(logger, "Test Setup Failed";
                           "error" => e.to_string(),
                           "test-name" => test.name(),
                           "category" => test.category());
            debug!(logger, "Backtrace";
                           "bt" => &format_args!("{}", e.backtrace()));

            failures.push(test);
        }
        Outcome::Fail(e) => {
            warn!(logger, "Test Failed";
                          "error" => e.to_string(),
                          "test-name" => test.name(),
                          "category" => test.category());
            debug!(logger, "Backtrace";
                           "bt" => &format_args!("{}", e.backtrace()));

            failures.push(test);
        }
        Outcome::ICE(msg) => {
            error!(logger, "The test case panicked!";
                           "msg" => msg,
                           "test-name" => test.name(),
                           "category" => test.category());

            failures.push(test);
        }
    }
}

fn interpret_panic_message(msg: Box<Any + Send + 'static>) -> Outcome {
    if let Some(msg) = msg.downcast_ref::<&str>() {
        Outcome::ICE(msg.to_string())
    } else if let Some(msg) = msg.downcast_ref::<String>() {
        Outcome::ICE(msg.clone())
    } else {
        Outcome::ICE("The test panicked".to_string())
    }
}

#[derive(Debug, Default, Clone)]
pub struct TestSuite {
    pub parse_fail: Vec<ParseFail>,
    pub run_pass: Vec<RunPass>,
}

impl TestSuite {
    pub fn tests(&self) -> impl Iterator<Item = &dyn TestCase> {
        self.parse_fail
            .iter()
            .map(|p| p as &dyn TestCase)
            .chain(self.run_pass.iter().map(|p| p as &dyn TestCase))
    }
}

fn load_test_suite(root: &Path, logger: &Logger) -> Result<TestSuite, Error> {
    info!(logger, "Loading test suite"; "root" => root.display());
    let mut suite = TestSuite::default();

    let parse_fail_dir = root.join("parse-fail");
    load(
        |p| ParseFail::for_file(p),
        &parse_fail_dir,
        &mut suite.parse_fail,
        logger,
    )?;

    let run_pass_dir = root.join("run-pass");
    load(
        |p| RunPass::for_file(p),
        &run_pass_dir,
        &mut suite.run_pass,
        logger,
    )?;

    debug!(logger, "Test suite loaded";
           "parse-fail-tests" => suite.parse_fail.len(),
           "run-pass-tests" => suite.run_pass.len());
    Ok(suite)
}

fn load<T, F>(
    mut constructor: F,
    dir: &Path,
    dest: &mut Vec<T>,
    logger: &Logger,
) -> Result<(), Error>
where
    T: TestCase,
    F: FnMut(&Path) -> Result<T, Error>,
{
    if dir.is_dir() {
        for entry in dir.read_dir()? {
            let entry = entry?;

            if entry.path().extension() == Some("c".as_ref()) {
                let path = entry.path();
                let test = constructor(&path)?;

                debug!(logger, "Found a test";
                       "filename" => path.display(),
                       "category" => test.category(),
                       "name" => test.name());
                dest.push(test);
            }
        }
    }

    Ok(())
}
