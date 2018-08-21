use codespan::CodeMap;
use crate::{Outcome, TestCase};
use failure::Error;
use mcc_driver::Driver;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct RunPass {
    filename: PathBuf,
}

impl RunPass {
    pub fn for_file<P: Into<PathBuf>>(filename: P) -> Result<RunPass, Error> {
        let filename = filename.into();

        if filename.exists() {
            Ok(RunPass { filename })
        } else {
            Err(failure::err_msg("The file doesn't exist").into())
        }
    }
}

impl TestCase for RunPass {
    fn run(&self) -> Outcome {
        let mut code_map = CodeMap::new();

        let fm = match code_map.add_filemap_from_disk(&self.filename) {
            Ok(f) => f,
            Err(e) => return Outcome::SetupFail(e.into()),
        };

        match Driver::new().run(&fm) {
            Ok(_) => Outcome::Pass,
            Err(diagnostics) => unimplemented!(),
        }
    }

    fn name(&self) -> &str {
        self.filename.file_stem().unwrap().to_str().unwrap()
    }

    fn category(&self) -> &str {
        "run-pass"
    }
}
