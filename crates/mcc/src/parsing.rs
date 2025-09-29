use codespan_reporting::diagnostic::Label;
use mcc_syntax::Span;
use tree_sitter::{Language, Node, StreamingIterator};

use crate::{
    Db, codes,
    diagnostics::{Diagnostic, DiagnosticExt},
    types::{Ast, SourceFile, Tree},
};

/// Parse a C program into an abstract syntax tree.
#[tracing::instrument(level = "info", skip_all)]
#[salsa::tracked]
pub fn parse(db: &dyn Db, file: SourceFile) -> Ast<'_> {
    let mut parser = tree_sitter::Parser::new();
    let lang = Language::new(tree_sitter_c::LANGUAGE);
    parser.set_language(&lang).unwrap();

    let src = file.contents(db);
    let tree = Tree::from(parser.parse(src, None).unwrap());

    check_tree(db, &tree, file);
    ensure_return_type(db, &lang, &tree, file);

    Ast::new(db, tree)
}

/// the return type for a C function is treated as optional by the grammar, but
/// we want it to be required.
#[tracing::instrument(level = "debug", skip_all)]
fn ensure_return_type(db: &dyn Db, lang: &Language, tree: &Tree, file: SourceFile) {
    let query = tree_sitter::Query::new(
        lang,
        "(function_definition
          type: (type_identifier) @missing-return-type
          declarator: (parenthesized_declarator)) @function-def",
    )
    .unwrap();
    let src = file.contents(db);

    let mut cursor = tree_sitter::QueryCursor::new();
    let mut captures = cursor.matches(&query, tree.root_node(), src.as_bytes());

    while let Some(m) = captures.next() {
        let diagnostic = codespan_reporting::diagnostic::Diagnostic::error()
            .with_message("Expected a return type for function")
            .with_code(codes::parse::missing_token)
            .with_labels(vec![
                Label::primary(file, Span::for_node(m.captures[0].node))
                    .with_message("error occurred here"),
            ]);
        diagnostic.accumulate(db);
    }
}

#[tracing::instrument(level = "debug", skip_all)]
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
    use codespan_reporting::diagnostic::Label;

    use crate::{Database, diagnostics::Diagnostics};

    use super::*;

    #[test]
    fn detect_missing_return_type() {
        let db = Database::default();

        let src = r#"
            // note: in older versions of C this would be valid
            // and return type would default to 'int'
            // GCC/Clang will compile it (with a warning)
            // for backwards compatibility
            main(void) {
                return 0;
            }
        "#;

        let file = SourceFile::new(&db, "test.c".into(), src.into());
        let diags = parse::accumulated::<Diagnostics>(&db, file);

        assert_eq!(
            diags,
            &[&Diagnostics::from(
                codespan_reporting::diagnostic::Diagnostic::error()
                    .with_code(codes::parse::missing_token)
                    .with_message("Expected a return type for function")
                    .with_labels(vec![
                        Label::primary(file, Span::new(232, 52))
                            .with_message("error occurred here")
                    ])
            )]
        );
    }
}
