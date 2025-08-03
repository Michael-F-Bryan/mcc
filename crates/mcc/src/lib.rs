pub extern crate target_lexicon;

mod assembling;
mod cmd;
pub mod codegen;
pub mod diagnostics;
mod files;
pub mod lowering;
mod parsing;
mod preprocessing;
mod text;
pub mod types;

pub use crate::{
    assembling::assemble_and_link, cmd::CommandError, files::Files, parsing::parse,
    preprocessing::preprocess, text::Text,
};

use target_lexicon::{Architecture, Triple};

#[salsa::db]
pub trait Db: salsa::Database {}

#[salsa::db]
impl<T: salsa::Database> Db for T {}

#[salsa::db]
#[derive(Default, Clone)]
pub struct Database {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for Database {}

pub fn default_target() -> Triple {
    Triple {
        architecture: Architecture::X86_64,
        ..Triple::host()
    }
}
