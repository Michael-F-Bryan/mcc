//! Michael's C Compiler — core library
//!
//! This crate implements the core compilation pipeline and data structures for
//! a small, testable C compiler. It is designed to be embedded in tools and
//! tests, and powers the `mcc` command-line tool in the `mcc-driver` crate.
//!
//! The architecture follows a classic pipeline, with each stage tracked by
//! [`salsa`] to enable incremental recomputation and diagnostics accumulation:
//!
//! - Preprocessing: [`preprocess`]
//! - Parsing: [`parse`]
//! - Lowering to TAC: [`lowering::lower`]
//! - Code generation (ASM IR): [`codegen::generate_assembly`]
//! - Rendering (assembly text): [`render::render_program`]
//! - Assembling and linking: [`assemble_and_link`]
//!
//! Data is passed through well-defined types like [`types::SourceFile`],
//! [`types::Ast`], [`codegen::asm::Program`], and [`Text`]. Diagnostics are
//! accumulated via a salsa accumulator, see [`diagnostics`]. Targets are
//! described using [`target_lexicon::Triple`].
//!
//! Quick start
//! -----------
//! Parse, lower, generate, and render a tiny program to assembly text:
//!
//! ```rust
//! use mcc::{Database, SourceFile, Text};
//!
//! let db = Database::default();
//! let src = "int main(void) { return 0; }";
//! let file = SourceFile::new(&db, Text::from("main.c"), Text::from(src));
//!
//! // Parse → TAC → ASM IR → assembly text
//! let ast = mcc::parse(&db, file);
//! let tacky = mcc::lowering::lower(&db, ast, file);
//! let asm_ir = mcc::codegen::generate_assembly(&db, tacky);
//! let asm_text = mcc::render_program(&db, asm_ir, mcc::default_target()).unwrap();
//!
//! assert!(asm_text.as_str().contains("main"));
//! ```
//!
//! Capturing diagnostics
//! ---------------------
//! Each stage can emit diagnostics which are accumulated rather than panicking.
//! You can retrieve them using the stage’s `accumulated::<T>()` helper:
//!
//! ```rust
//! use mcc::{Database, SourceFile, Text, diagnostics::Diagnostics};
//!
//! let db = Database::default();
//! let file = SourceFile::new(&db, "test.c".into(), "int main(void) {}".into());
//! let _ = mcc::parse(&db, file);
//! let diags: Vec<&Diagnostics> = mcc::parse::accumulated::<Diagnostics>(&db, file);
//! // Render with codespan-reporting using `mcc::Files`
//! ```
//!
//! Targets and OS-specific behavior
//! --------------------------------
//! Rendering takes a [`target_lexicon::Triple`]. Use [`default_target()`] for a
//! reasonable default. On macOS, symbol names are rendered with a leading
//! underscore (e.g., `_main`); on Linux, a `.note.GNU-stack` section is emitted.
//!
//! Notes on preprocessing
//! ----------------------
//! [`preprocess`] invokes the system C compiler’s preprocessor (e.g. `cc -E -P`).
//! The `mcc` driver currently runs preprocessing and writes the result to a
//! temporary file; the parser reads the original [`SourceFile`] contents.
//!
//! See also
//! --------
//! - [`codegen::asm`] for the assembly IR
//! - [`diagnostics`] for diagnostics accumulation and error codes
//! - [`mcc-driver`] for CLI orchestration and staged callbacks
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
mod render;
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
    render::render_program,
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
