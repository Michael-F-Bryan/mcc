//! Michael's C Compiler — driver library
//!
//! This crate provides the command-line interface and orchestration for the
//! `mcc` compiler. It wires together the core stages from the `mcc` crate,
//! manages temporary files, captures diagnostics between stages, and exposes a
//! small callback API for tooling and testing.
//!
//! What this crate offers
//! ----------------------
//! - A `main()` entrypoint for the `mcc` binary: [`cli::main`]
//! - A stage runner with callbacks: [`run`] and the [`Callbacks`] trait
//! - A configuration object for compilation sessions: [`Config`]
//! - An outcome type capturing success, error, or early return: [`Outcome`]
//!
//! Running the pipeline with callbacks
//! -----------------------------------
//! The [`run`] function executes the full pipeline:
//! preprocessing → parsing → lowering → codegen → rendering → assemble+link.
//!
//! You can implement [`Callbacks`] to observe intermediate artifacts, collect
//! diagnostics, or stop the pipeline early:
//!
//! ```rust
//! use std::ops::ControlFlow;
//! use mcc_driver::{Callbacks, Config, Outcome};
//! use mcc::{Ast, Text, diagnostics::Diagnostics, codegen::asm, lowering::tacky};
//!
//! struct Noop;
//! impl Callbacks for Noop {
//!     type Output = ();
//!
//!     fn after_parse<'db>(
//!         &mut self,
//!         _db: &'db dyn mcc::Db,
//!         _source_file: mcc::SourceFile,
//!         _ast: Ast<'db>,
//!         _diags: Vec<&Diagnostics>,
//!     ) -> ControlFlow<Self::Output> {
//!         ControlFlow::Continue(())
//!     }
//!
//!     fn after_lower<'db>(
//!         &mut self,
//!         _db: &'db dyn mcc::Db,
//!         _tacky: tacky::Program<'db>,
//!         _diags: Vec<&Diagnostics>,
//!     ) -> ControlFlow<Self::Output> { ControlFlow::Continue(()) }
//!
//!     fn after_codegen<'db>(
//!         &mut self,
//!         _db: &'db dyn mcc::Db,
//!         _asm: asm::Program<'db>,
//!         _diags: Vec<&Diagnostics>,
//!     ) -> ControlFlow<Self::Output> { ControlFlow::Continue(()) }
//!
//!     fn after_render_assembly(
//!         &mut self,
//!         _db: &dyn mcc::Db,
//!         _asm: Text,
//!         _diags: Vec<&Diagnostics>,
//!     ) -> ControlFlow<Self::Output> { ControlFlow::Continue(()) }
//! }
//!
//! // In your application:
//! // let outcome = mcc_driver::run(&mut Noop, config);
//! // outcome.to_result()?;
//! ```
//!
//! CLI entrypoint
//! --------------
//! The binary uses [`cli::main`] to parse flags (target triple, output path,
//! color, stop-at-stage switches, etc.), set up tracing, and delegate to
//! [`run`]. See `crates/mcc-driver/src/cli.rs` for details.
//!

mod callbacks;
mod cli;

pub use crate::{
    callbacks::{Callbacks, Config, Outcome, run},
    cli::main,
};
