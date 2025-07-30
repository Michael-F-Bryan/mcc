mod span;

pub use crate::span::Span;

/// A strongly-typed AST for C, built using [`type-sitter`].
#[allow(clippy::all)]
pub mod ast {
    include!(concat!(env!("OUT_DIR"), "/nodes.rs"));
}
