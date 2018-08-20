//! The top level "main function" for `mcc`.

#![warn(rust_2018_idioms)]

#[macro_use]
extern crate slog;

mod timer;

use codespan::FileMap;
use codespan_reporting::Severity;
use crate::timer::Timer;
use mcc::hir::CompilationUnit;
use mcc::Diagnostics;
use slog::{Discard, Logger};
use std::mem;
use syntax;
use syntax::ast::File;

#[derive(Debug)]
pub struct Driver {
    logger: Logger,
    timer: Timer,
    diags: Diagnostics,
}

impl Driver {
    pub fn new() -> Driver {
        Driver::new_with_logger(Logger::root(Discard, o!()))
    }

    pub fn new_with_logger(logger: Logger) -> Driver {
        Driver {
            timer: Timer::new(&logger),
            diags: Diagnostics::new(),
            logger,
        }
    }

    pub fn run(&mut self, map: &FileMap) -> Result<(), Diagnostics> {
        info!(self.logger, "Started compilation process";
              "filename" => &format_args!("{}", map.name()));

        self.timer.start("parse");
        let ast = self.parse(map)?;
        self.timer.log_memory_usage(&[&ast, &self.diags]);
        self.timer.pop();

        self.timer.start("translation");
        let hir = self.trans(&ast)?;
        self.timer.log_memory_usage(&[&hir, &ast, &self.diags]);
        self.timer.pop();

        unimplemented!()
    }

    fn parse(&mut self, map: &FileMap) -> Result<File, Diagnostics> {
        match syntax::parse(map) {
            Ok(ast) => Ok(ast),
            Err(diag) => {
                let mut diags = self.swap_diags();
                diags.add(diag);
                self.timer.cancel();
                Err(diags)
            }
        }
    }

    fn trans(&mut self, ast: &File) -> Result<CompilationUnit, Diagnostics> {
        let hir = mcc::translate(ast, &mut self.diags);

        if self.diags.has_errors() {
            info!(self.logger, "Aborting translation";
                  "errors" => self.diags.diagnostics_more_severe_than(Severity::Error));
            self.timer.cancel();
            Err(self.swap_diags())
        } else {
            Ok(hir)
        }
    }

    fn swap_diags(&mut self) -> Diagnostics {
        mem::replace(&mut self.diags, Diagnostics::new())
    }
}
