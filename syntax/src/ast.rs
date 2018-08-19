use codespan::ByteSpan;
use crate::node_id::NodeId;
use heapsize::HeapSizeOf;
use heapsize_derive::HeapSizeOf;
use std::any::Any;
use sum_type::*;

pub trait AstNode: Any + HeapSizeOf {
    fn span(&self) -> ByteSpan;
    fn node_id(&self) -> NodeId;
}

/// The result of parsing an entire source file.
#[derive(Debug, Clone, PartialEq, HeapSizeOf)]
pub struct File {
    pub span: ByteSpan,
    pub node_id: NodeId,
    pub items: Vec<Item>,
}

impl File {
    pub(crate) fn new(items: Vec<Item>, span: ByteSpan) -> File {
        File {
            items,
            span,
            node_id: NodeId::placeholder(),
        }
    }
}

/// A top-level item.
#[derive(Debug, Clone, PartialEq, HeapSizeOf)]
pub enum Item {
    Function(Function),
}

/// A function definition.
#[derive(Debug, Clone, PartialEq, HeapSizeOf)]
pub struct Function {
    pub span: ByteSpan,
    pub node_id: NodeId,
    pub signature: FnDecl,
    pub body: Vec<Statement>,
}

/// A function signature.
#[derive(Debug, Clone, PartialEq, HeapSizeOf)]
pub struct FnDecl {
    pub span: ByteSpan,
    pub node_id: NodeId,
    pub return_value: Option<Type>,
    pub name: Ident,
    pub args: Vec<Argument>,
}

/// An identifier.
#[derive(Debug, Clone, PartialEq, HeapSizeOf)]
pub struct Ident {
    pub span: ByteSpan,
    pub node_id: NodeId,
    pub name: String,
}

/// A type.
#[derive(Debug, Clone, PartialEq, HeapSizeOf)]
pub enum Type {
    Ident(Ident),
}

/// A return statement.
#[derive(Debug, Clone, PartialEq, HeapSizeOf)]
pub struct Return {
    pub span: ByteSpan,
    pub node_id: NodeId,
    pub value: Option<Expression>,
}

impl Return {
    pub(crate) fn bare(span: ByteSpan) -> Return {
        Return {
            span,
            node_id: NodeId::placeholder(),
            value: None,
        }
    }

    pub(crate) fn value(value: Expression, span: ByteSpan) -> Return {
        Return {
            span,
            node_id: NodeId::placeholder(),
            value: Some(value),
        }
    }
}

sum_type! {
    /// Any statement.
    #[derive(Debug, Clone, PartialEq, HeapSizeOf)]
    pub enum Statement {
        Return,
        /// Dummy variant so we can use the `sum_type!()` macro.
        u32,
    }
}

sum_type! {
    /// Any expression.
    #[derive(Debug, Clone, PartialEq, HeapSizeOf)]
    pub enum Expression {
        Literal,
        BinaryOp,
    }
}

#[derive(Debug, Clone, PartialEq, HeapSizeOf)]
pub struct BinaryOp {
    pub span: ByteSpan,
    pub node_id: NodeId,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
    pub kind: BinaryOperator,
}

#[derive(Debug, Copy, Clone, PartialEq, HeapSizeOf)]
pub enum BinaryOperator {
    Add,
    Subtract,
}

/// A single function argument.
#[derive(Debug, Clone, PartialEq, HeapSizeOf)]
pub struct Argument {
    pub span: ByteSpan,
    pub node_id: NodeId,
    pub name: Option<Ident>,
    pub ty: Type,
}

/// A literal value.
#[derive(Debug, Clone, PartialEq, HeapSizeOf)]
pub struct Literal {
    pub span: ByteSpan,
    pub node_id: NodeId,
    pub kind: LiteralKind,
}

impl Literal {
    pub(crate) fn new(kind: LiteralKind, span: ByteSpan) -> Literal {
        Literal {
            kind,
            span,
            node_id: NodeId::placeholder(),
        }
    }
}

sum_type! {
    /// The particular kind of literal.
    #[derive(Debug, Clone, PartialEq, HeapSizeOf)]
    pub enum LiteralKind {
        Float(f64),
        Integer(i64),
        String(String),
    }
}

macro_rules! impl_ast_node {
    ($type:ty) => {
        impl $crate::ast::AstNode for $type {
            fn span(&self) -> ByteSpan {
                self.span
            }

            fn node_id(&self) -> NodeId {
                self.node_id
            }
        }
    };
}

impl_ast_node!(Argument);
impl_ast_node!(File);
impl_ast_node!(FnDecl);
impl_ast_node!(Function);
impl_ast_node!(Ident);
impl_ast_node!(Literal);
impl_ast_node!(Return);
