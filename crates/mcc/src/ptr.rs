//! Salsa-friendly pointers to a [`tree_sitter::Node`] inside a [`SourceFile`].

use std::ops::Deref;

use mcc_syntax::Span;
use type_sitter::Node;

use crate::{Db, SourceFile};

/// An untyped reference to a [`tree_sitter::Node`] in a [`SourceFile`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct RawPtr {
    pub file: SourceFile,
    pub span: Span,
    pub id: usize,
}

impl RawPtr {
    pub fn for_node(file: SourceFile, node: tree_sitter::Node<'_>) -> Self {
        let span = Span::for_node(node);

        RawPtr {
            file,
            span,
            id: node.id(),
        }
    }

    pub fn node<'db>(&self, db: &'db dyn Db) -> tree_sitter::Node<'db> {
        let tree = self.file.ast(db).tree(db);
        let mut cursor = tree.walk();

        loop {
            let node = cursor.node();

            if node.id() == self.id {
                return node;
            }

            if cursor.goto_first_child_for_byte(self.span.start).is_none() {
                panic!("failed to find node with id {}", self.id);
            }
        }
    }
}

/// A strongly-typed pointer to a [`type_sitter::Node`] inside a [`SourceFile`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Ptr<T> {
    ptr: RawPtr,
    _node_type: std::marker::PhantomData<T>,
}

impl<T: type_sitter::Node<'static>> Ptr<T> {
    pub fn for_node<'tree>(file: SourceFile, node: T::WithLifetime<'tree>) -> Self {
        let node = node.raw();
        Ptr {
            ptr: RawPtr::for_node(file, *node),
            _node_type: std::marker::PhantomData,
        }
    }

    pub fn node<'db>(self, db: &'db dyn Db) -> T::WithLifetime<'db> {
        let node = self.ptr.node(db);
        T::WithLifetime::<'db>::try_from_raw(node).unwrap()
    }

    pub fn raw(&self) -> RawPtr {
        self.ptr
    }
}

impl<'tree, T: type_sitter::Node<'tree>> Deref for Ptr<T> {
    type Target = RawPtr;
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

macro_rules! ptr_type {
    (
        $(
            $(#[$meta:meta])*
            pub struct $name:ident($node:ident);
        )*
    ) => {
        $(
            $(#[$meta])*
            #[doc = concat!("A pointer to a [`mcc_syntax::ast::", stringify!($node), "`] node.")]
            #[salsa::interned]
            #[derive(Debug)]
            pub struct $name<'db> {
                pub ptr: $crate::ptr::Ptr<mcc_syntax::ast::$node<'static>>,
            }

            impl<'db> $name<'db> {
                pub fn from_node(db: &'db dyn $crate::Db, source_file: $crate::types::SourceFile, node: mcc_syntax::ast::$node<'db>) -> Self {
                    let ptr = $crate::ptr::Ptr::for_node(source_file, node);
                    $name::new(db, ptr)
                }

                pub fn node(&self, db: &'db dyn $crate::Db) -> mcc_syntax::ast::$node<'db> {
                    self.ptr(db).node(db)
                }

                pub fn file(&self, db: &'db dyn $crate::Db) -> $crate::types::SourceFile {
                    self.ptr(db).file
                }

                pub fn span(&self, db: &'db dyn $crate::Db) -> mcc_syntax::Span {
                    self.ptr(db).span
                }
            }
        )*
    };
}

ptr_type! {
    pub struct FunctionDefinition(FunctionDefinition);
    pub struct Declaration(Declaration);
}

#[cfg(test)]
mod tests {
    use tree_sitter::{Query, QueryCursor, StreamingIterator};

    use super::*;

    #[test]
    fn round_trip_ptr() {
        let db = salsa::DatabaseImpl::default();
        let file = SourceFile::new(&db, "test.c".into(), "int main(void) {}".into());
        let tree = file.ast(&db).tree(&db);

        let lang = tree_sitter::Language::new(tree_sitter_c::LANGUAGE);
        let query = Query::new(&lang, "(function_definition) @f").unwrap();
        let mut cursor = QueryCursor::new();
        let mut captures = cursor.captures(&query, tree.root_node(), file.contents(&db).as_bytes());
        let (matches, idx) = captures.next().unwrap();
        let node = matches.captures[*idx].node;

        let ptr = dbg!(RawPtr::for_node(file, node));
        let round_tripped = ptr.node(&db);
        assert_eq!(round_tripped, node);
    }
}
