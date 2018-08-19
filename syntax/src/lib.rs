#![feature(rust_2018_preview)]
#![warn(rust_2018_idioms)]

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

pub mod ast;
#[allow(dead_code, rust_2018_idioms)]
mod grammar;
mod node_id;
mod parse;

pub use self::node_id::NodeId;
pub use self::parse::parse;
