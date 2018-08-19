#![feature(rust_2018_preview)]
#![warn(rust_2018_idioms)]

pub mod ast;
mod node_id;
mod parse;

pub use self::node_id::NodeId;
