mod assembling;
mod cmd;
mod compiling;
pub mod diagnostics;
mod files;
mod parsing;
mod preprocessing;
mod text;
pub mod types;

pub use crate::{
    assembling::{AssemblyInput, assemble_and_link},
    cmd::CommandError,
    compiling::compile,
    files::Files,
    parsing::parse,
    preprocessing::preprocess,
    text::Text,
};

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
