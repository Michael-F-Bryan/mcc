//! The High-Level Intermediate Representation (HIR) for the C compiler.
//!
//! While the [`mcc_syntax::ast`] module provides a strongly-typed concrete syntax
//! tree that perfectly represents the source code (syntax errors and all), the
//! `hir` is a refined representation of the program that is easier to work with
//! for the subsequent stages of the compiler.

use mcc_syntax::Span;
use type_sitter::Node;

use crate::{Db, SourceFile, Text, ptr};

#[salsa::tracked(debug)]
pub struct TranslationUnit<'db> {
    #[returns(ref)]
    pub items: Vec<Item<'db>>,
    pub file: SourceFile,
}

/// A named top-level item.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, salsa::Update)]
pub enum Item<'db> {
    Function(FunctionDefinition<'db>),
}

impl<'db> Item<'db> {
    pub fn name(&self, db: &'db dyn Db) -> Identifier<'db> {
        match self {
            Item::Function(f) => f.name(db),
        }
    }
}

#[salsa::tracked(debug)]
pub struct FunctionDefinition<'db> {
    pub name: Identifier<'db>,
    pub node: ptr::FunctionDefinition<'db>,
}

#[salsa::interned(debug)]
#[derive(PartialOrd, Ord)]
pub struct Identifier {
    pub text: Text,
}

impl<'db> Identifier<'db> {
    pub fn from_node(
        db: &'db dyn Db,
        file: SourceFile,
        node: mcc_syntax::ast::Identifier<'db>,
    ) -> Self {
        let src = file.contents(db);
        let name = node.utf8_text(src.as_bytes()).expect("unreachable");
        Identifier::new(db, Text::from(name))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, salsa::Update)]
pub enum Type {
    Void,
    Int,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, salsa::Update)]
pub struct Signature<'db> {
    pub return_type: Type,
    pub parameters: Vec<Parameter<'db>>,
}

#[salsa::tracked(debug)]
pub struct Parameter<'db> {
    pub name: Identifier<'db>,
    pub type_: Type,
    pub node: ptr::Declaration<'db>,
}

#[salsa::tracked(debug)]
pub struct VariableDefinition<'db> {
    pub name: Identifier<'db>,
    pub type_: Type,
    pub node: ptr::Declaration<'db>,
}

/// The definition of a symbol in the current scope.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, salsa::Update)]
pub enum Def<'db> {
    Function(FunctionDefinition<'db>),
    Parameter(Parameter<'db>),
}

impl<'db> Def<'db> {
    pub fn name(&self, db: &'db dyn Db) -> Identifier<'db> {
        match self {
            Def::Function(f) => f.name(db),
            Def::Parameter(p) => p.name(db),
        }
    }

    pub fn ptr(&self, db: &'db dyn Db) -> ptr::RawPtr {
        match self {
            Def::Function(f) => f.node(db).ptr(db).raw(),
            Def::Parameter(p) => p.node(db).ptr(db).raw(),
        }
    }

    pub fn span(&self, db: &'db dyn Db) -> Span {
        self.ptr(db).span
    }
}
