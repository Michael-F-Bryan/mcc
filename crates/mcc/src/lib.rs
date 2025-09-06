pub extern crate target_lexicon;

mod assembling;
mod cmd;
pub mod codegen;
mod debug;
pub mod diagnostics;
mod files;
pub mod lowering;
mod parsing;
mod preprocessing;
pub mod render;
mod text;
mod types;

use std::fmt::{self, Debug};

pub use crate::{
    assembling::assemble_and_link,
    cmd::CommandError,
    codegen::generate_assembly,
    debug::SerializeWithDatabase,
    files::Files,
    lowering::lower,
    parsing::parse,
    preprocessing::preprocess,
    text::Text,
    types::{Ast, SourceFile, Tree},
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

impl Debug for Database {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Database { storage: _ } = self;

        f.debug_struct("Database").finish_non_exhaustive()
    }
}

pub fn default_target() -> Triple {
    Triple {
        architecture: Architecture::X86_64,
        ..Triple::host()
    }
}
