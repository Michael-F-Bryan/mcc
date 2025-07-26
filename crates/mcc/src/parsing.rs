use salsa::Accumulator;
use tree_sitter::{Language, Node};

use crate::{
    Db,
    diagnostics::{Diagnostic, ParseError},
    types::{Ast, SourceFile, Span, Tree},
};

/// Parse a C program into an abstract syntax tree.
///
///
#[salsa::tracked]
pub fn parse(db: &dyn Db, file: SourceFile) -> Ast<'_> {
    let mut parser = tree_sitter::Parser::new();
    let lang = Language::new(tree_sitter_c::LANGUAGE);
    parser.set_language(&lang).unwrap();

    let src = file.contents(db);
    let tree = Tree::from(parser.parse(src, None).unwrap());

    check_tree(db, &tree, file);

    Ast::new(db, tree)
}

fn check_tree(db: &dyn Db, tree: &Tree, file: SourceFile) {
    let mut cursor = tree.walk();

    let mut to_check: Vec<Node<'_>> = vec![tree.root_node()];

    while let Some(node) = to_check.pop() {
        match check_node(db, node, file) {
            Continuation::Skip => {}
            Continuation::Recurse => {
                cursor.reset(node);
                for child in node.children(&mut cursor) {
                    to_check.push(child);
                }
            }
            Continuation::Emit(diag) => {
                diag.accumulate(db);
            }
        }
    }
}

fn check_node(db: &dyn Db, node: Node<'_>, file: SourceFile) -> Continuation {
    if !node.has_error() {
        Continuation::Skip
    } else if node.is_missing() {
        let range = node.byte_range();
        let err_span = Span::new(range.start, range.end - range.start);

        let error = ParseError {
            file,
            msg: format!("Expected a \"{}\"", node.parent().unwrap().grammar_name(),).into(),
            span: err_span,
        };
        Continuation::Emit(error.into())
    } else if node.is_error() {
        let range = node.byte_range();
        let err_span = Span::new(range.start, range.end - range.start);
        let token = node.utf8_text(file.contents(db).as_ref()).unwrap();

        let error = ParseError {
            file,
            msg: format!(
                "Expected a \"{}\", but found \"{}\"",
                node.parent().unwrap().grammar_name(),
                token
            )
            .into(),
            span: err_span,
        };
        Continuation::Emit(error.into())
    } else {
        Continuation::Recurse
    }
}

#[derive(Debug, Clone)]
enum Continuation {
    /// Skip this node and all its children.
    Skip,
    /// Recurse into the children of this node.
    Recurse,
    /// Emit a diagnostic for this node.
    Emit(Diagnostic),
}
