#[macro_use]
extern crate slog;

mod parse_fail;
mod run_pass;
mod runner;

pub use crate::parse_fail::ParseFail;
pub use crate::run_pass::RunPass;
pub use crate::runner::run;

use failure::Error;
use std::panic::RefUnwindSafe;

pub trait TestCase: RefUnwindSafe {
    fn run(&self) -> Outcome;
    fn name(&self) -> &str;
    fn category(&self) -> &str;
}

#[derive(Debug)]
pub enum Outcome {
    Pass,
    Fail(Error),
    SetupFail(Error),
    ICE(String),
}
