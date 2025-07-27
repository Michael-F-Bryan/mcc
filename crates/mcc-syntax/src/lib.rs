mod span;

pub use crate::span::Span;

pub mod nodes {
    include!(concat!(env!("OUT_DIR"), "/nodes.rs"));
}

pub trait Spanned {
    fn span(&self) -> Span;
}

impl<'tree, T: type_sitter::Node<'tree>> Spanned for T {
    fn span(&self) -> Span {
        Span::for_node(*self.raw())
    }
}
