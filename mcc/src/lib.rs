//! High level IR, typechecking, and code generation for `mcc`.

#![warn(rust_2018_idioms)]

mod diagnostics;
pub mod hir;
mod trans;

pub use crate::diagnostics::Diagnostics;
pub use crate::trans::translate;
