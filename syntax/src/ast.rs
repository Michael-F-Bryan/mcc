use codespan::ByteSpan;
use crate::node_id::NodeId;
use heapsize::HeapSizeOf;
use heapsize_derive::HeapSizeOf;
use std::any::Any;
use std::fmt::Debug;
use sum_type::*;

pub trait AstNode: Any + HeapSizeOf + Debug {
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

sum_type! {
    /// A top-level item.
    #[derive(Debug, Clone, PartialEq, HeapSizeOf)]
    pub enum Item {
        Function,
        u32,
    }
}

/// A function definition.
#[derive(Debug, Clone, PartialEq, HeapSizeOf)]
pub struct Function {
    pub span: ByteSpan,
    pub node_id: NodeId,
    pub signature: FnDecl,
    pub body: Vec<Statement>,
}

impl Function {
    pub(crate) fn new(signature: FnDecl, body: Vec<Statement>, span: ByteSpan) -> Function {
        Function {
            span,
            signature,
            body,
            node_id: NodeId::placeholder(),
        }
    }

    pub fn name(&self) -> &str {
        &self.signature.name.name
    }
}

/// A function signature.
#[derive(Debug, Clone, PartialEq, HeapSizeOf)]
pub struct FnDecl {
    pub span: ByteSpan,
    pub node_id: NodeId,
    pub return_value: Type,
    pub name: Ident,
    pub args: Vec<Argument>,
}

impl FnDecl {
    pub(crate) fn new(
        name: Ident,
        return_value: Type,
        args: Vec<Argument>,
        span: ByteSpan,
    ) -> FnDecl {
        FnDecl {
            name,
            return_value,
            args,
            span,
            node_id: NodeId::placeholder(),
        }
    }
}

/// An identifier.
#[derive(Debug, Clone, PartialEq, HeapSizeOf)]
pub struct Ident {
    pub span: ByteSpan,
    pub node_id: NodeId,
    pub name: String,
}

impl Ident {
    pub(crate) fn new(name: &str, span: ByteSpan) -> Ident {
        Ident {
            span,
            name: name.into(),
            node_id: NodeId::placeholder(),
        }
    }
}

/// A type.
#[derive(Debug, Clone, PartialEq, HeapSizeOf)]
pub enum Type {
    Ident(Ident),
}

impl From<Ident> for Type {
    fn from(other: Ident) -> Type {
        Type::Ident(other)
    }
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

/// Apply the same operation to each variant in an enum.
///
/// # Examples
///
/// ```rust
/// # #[macro_use]
/// # extern crate syntax;
/// #[derive(Debug)]
/// pub enum Foo {
///     First(u32),
///     Second(String),
///     Third(Box<Foo>),
/// }
///
/// # fn main() {
/// let foo = Foo::Second("Hello World!".into());
///
/// defer!(Foo, foo; First, Second, Third => |item| println!("{:?}", item));
/// // prints "Hello World"
/// # }
/// ```
#[macro_export]
macro_rules! defer {
    ($type:ident, $this:expr; $( $variant:tt ),* => |$item:ident| $process_item:expr) => {{
        use self::$type::*;

        #[allow(unreachable_patterns)]
        match $this {
            $(
                $variant(ref $item) => $process_item,
            )*
            _ => unreachable!(),
        }
    }};
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

    ($type:ident; $( $variant:ident ),+) => {
        impl $crate::ast::AstNode for $type {
            fn span(&self) -> ByteSpan {
                defer!($type, self; $($variant),+ => |item| item.span())
            }

            fn node_id(&self) -> NodeId {
                defer!($type, self; $($variant),+ => |item| item.node_id())
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
impl_ast_node!(BinaryOp);
impl_ast_node!(Item; Function);
impl_ast_node!(Statement; Return);
impl_ast_node!(Expression; Literal, BinaryOp);
impl_ast_node!(Type; Ident);
