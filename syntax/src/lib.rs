//! A basic parser and AST.

#![warn(rust_2018_idioms)]

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

pub mod ast;
mod grammar;
mod node_id;
mod parse;
pub mod visitor;

pub use self::node_id::NodeId;
pub use self::parse::parse;
