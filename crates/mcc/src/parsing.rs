use codespan_reporting::diagnostic::Label;
use mcc_syntax::Span;
use tree_sitter::Node as TsNode;

use crate::{
    Db, codes,
    diagnostics::{Diagnostic, DiagnosticExt},
    types::{Ast, SourceFile, Tree},
};

/// Parse a C program into an abstract syntax tree.
/// Only syntax (tree shape) is checked here; semantic checks (return type, keywords, type specifiers) are done in typechecking.
#[tracing::instrument(level = "info", skip_all)]
#[salsa::tracked]
pub fn parse(db: &dyn Db, file: SourceFile) -> Ast<'_> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter::Language::new(tree_sitter_c::LANGUAGE))
        .unwrap();

    let src = file.contents(db);
    let tree = Tree::from(parser.parse(src, None).unwrap());

    check_tree(db, &tree, file);

    Ast::new(db, tree)
}

#[tracing::instrument(level = "debug", skip_all)]
fn check_tree(db: &dyn Db, tree: &Tree, file: SourceFile) {
    let mut cursor = tree.walk();

    let mut to_check: Vec<TsNode<'_>> = vec![tree.root_node()];

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

fn check_node(db: &dyn Db, node: TsNode<'_>, file: SourceFile) -> Continuation {
    if !node.has_error() {
        Continuation::Skip
    } else if node.is_missing() {
        let diagnostic = Diagnostic::error()
            .with_message(format!(
                "Expected a \"{}\"",
                node.parent().unwrap().grammar_name()
            ))
            .with_code(codes::parse::unexpected_token)
            .with_labels(vec![
                Label::primary(file, Span::for_node(node)).with_message("error occurred here"),
            ]);
        Continuation::Emit(diagnostic)
    } else if node.is_error() {
        let token = node.utf8_text(file.contents(db).as_ref()).unwrap();

        let diagnostic = Diagnostic::error()
            .with_message(format!(
                "Expected a \"{}\", but found \"{}\"",
                node.parent().unwrap().grammar_name(),
                token
            ))
            .with_labels(vec![
                Label::primary(file, Span::for_node(node)).with_message("error occurred here"),
            ]);
        Continuation::Emit(diagnostic)
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

#[cfg(test)]
mod tests {
    use crate::{Database, types::SourceFile};

    use super::*;

    #[test]
    fn parse_produces_ast_for_valid_program() {
        let db = Database::default();
        let src = "int main(void) { return 0; }";
        let file = SourceFile::new(&db, "test.c".into(), src.into());
        let _ast = parse(&db, file);
        assert!(parse::accumulated::<crate::diagnostics::Diagnostics>(&db, file).is_empty());
    }
}
