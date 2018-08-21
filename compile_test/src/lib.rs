#[macro_use]
extern crate slog;

mod parse_fail;
mod runner;

pub use crate::parse_fail::ParseFail;
pub use crate::runner::run;

use failure::Error;

pub trait TestCase {
    fn run(&self) -> Outcome;
    fn name(&self) -> &str;
}

#[derive(Debug)]
pub enum Outcome {
    Pass,
    Fail(Error),
    SetupFail(Error),
}
