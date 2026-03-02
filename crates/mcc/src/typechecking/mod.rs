use codespan_reporting::diagnostic::Label;
use mcc_syntax::{Span, ast};
use tree_sitter::{Language, Node as TsNode, StreamingIterator};
use type_sitter::{HasChildren, Node};

use crate::{
    Db, SourceFile, codes,
    diagnostics::{Diagnostic, DiagnosticExt, Diagnostics},
    ptr,
};

const C_KEYWORDS: &[&str] = &[
    "auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else",
    "enum", "extern", "float", "for", "goto", "if", "int", "long", "register", "return", "short",
    "signed", "sizeof", "static", "struct", "switch", "typedef", "union", "unsigned", "void",
    "volatile", "while",
];

pub mod hir;

#[salsa::tracked]
#[tracing::instrument(level = "info", skip_all)]
pub fn typecheck<'db>(db: &'db dyn Db, file: SourceFile) -> hir::TranslationUnit<'db> {
    let ast = crate::parse(db, file);
    if !crate::parse::accumulated::<Diagnostics>(db, file).is_empty() {
        return hir::TranslationUnit::new(db, Vec::new(), file);
    }

    ensure_no_keyword_as_identifier(
        db,
        file,
        ast.tree(db).root_node(),
        file.contents(db).as_ref(),
    );

    let tu = ast.root(db);
    let mut cursor = tu.walk();
    let mut items = Vec::new();

    for child in tu.children(&mut cursor).filter_map(|c| c.ok()) {
        type Child<'db> = <ast::TranslationUnit<'db> as HasChildren<'db>>::Child;
        match child {
            Child::FunctionDefinition(f) => {
                if let Ok(body) = f.body() {
                    ensure_valid_declaration_types(db, file, body);
                }
                if let Some(hir_f) = function_definition(db, file, f) {
                    let _ = function_signature(db, hir_f);
                    items.push(hir::Item::Function(hir_f));
                }
            }
            other => {
                Diagnostic::bug()
                    .with_message("Translation unit item not implemented")
                    .with_code(codes::type_check::unimplemented)
                    .with_labels(vec![
                        Label::primary(file, Span::for_node(*other.raw()))
                            .with_message(other.kind()),
                    ])
                    .accumulate(db);
            }
        }
    }

    hir::TranslationUnit::new(db, items, file)
}

/// Lazily compute a function's signature (return type and parameters) from the AST.
/// Emits diagnostics for missing return type or invalid type specifier.
#[salsa::tracked]
fn function_signature<'db>(
    db: &'db dyn Db,
    f: hir::FunctionDefinition<'db>,
) -> hir::Signature<'db> {
    let node = f.node(db).node(db);
    let file = f.node(db).file(db);
    let src = file.contents(db);
    let raw = node.raw();

    let return_type = match raw.child_by_field_name("type") {
        None => {
            Diagnostic::error()
                .with_message("Expected a return type for function")
                .with_code(codes::type_check::missing_return_type)
                .with_labels(vec![
                    Label::primary(file, mcc_syntax::Span::for_node(*raw))
                        .with_message("error occurred here"),
                ])
                .accumulate(db);
            hir::Type::Error
        }
        Some(spec) => {
            let kind = spec.kind();
            let text = spec.utf8_text(src.as_bytes()).unwrap_or_default().trim();
            let is_old_style = raw
                .child_by_field_name("declarator")
                .map_or(false, |d| d.kind() == "parenthesized_declarator");
            if kind == "type_identifier" {
                if is_old_style {
                    Diagnostic::error()
                        .with_message("Expected a return type for function")
                        .with_code(codes::type_check::missing_return_type)
                        .with_labels(vec![
                            Label::primary(file, mcc_syntax::Span::for_node(spec))
                                .with_message("error occurred here"),
                        ])
                        .accumulate(db);
                } else {
                    Diagnostic::error()
                        .with_message(format!("invalid type \"{text}\""))
                        .with_code(codes::type_check::invalid_type)
                        .with_labels(vec![
                            Label::primary(file, mcc_syntax::Span::for_node(spec))
                                .with_message("invalid or unsupported type"),
                        ])
                        .accumulate(db);
                }
                hir::Type::Error
            } else if is_old_style && text == "void" {
                // Old-style main(void): type slot holds parameter's void, not return type.
                Diagnostic::error()
                    .with_message("Expected a return type for function")
                    .with_code(codes::type_check::missing_return_type)
                    .with_labels(vec![
                        Label::primary(file, mcc_syntax::Span::for_node(spec))
                            .with_message("error occurred here"),
                    ])
                    .accumulate(db);
                hir::Type::Error
            } else {
                match text {
                    "int" => hir::Type::Int,
                    "void" => hir::Type::Void,
                    _ => {
                        Diagnostic::error()
                            .with_message(format!("invalid type \"{text}\""))
                            .with_code(codes::type_check::invalid_type)
                            .with_labels(vec![
                                Label::primary(file, mcc_syntax::Span::for_node(spec))
                                    .with_message("invalid or unsupported type"),
                            ])
                            .accumulate(db);
                        hir::Type::Error
                    }
                }
            }
        }
    };

    let parameters = Vec::new();

    hir::Signature {
        return_type,
        parameters,
    }
}

fn function_definition<'db>(
    db: &'db dyn Db,
    file: SourceFile,
    f: ast::FunctionDefinition<'db>,
) -> Option<hir::FunctionDefinition<'db>> {
    let signature: ast::FunctionDeclarator<'db> = f.declarator().ok()?.as_function_declarator()?;
    let ident: ast::Identifier<'db> = signature.declarator().ok()?.as_identifier()?;

    let name = hir::Identifier::from_node(db, file, ident);
    let ptr = ptr::FunctionDefinition::from_node(db, file, f);

    Some(hir::FunctionDefinition::new(db, name, ptr))
}

/// C keywords may not be used as identifiers (e.g. `int return = 4;`).
/// Only flag identifiers used as names (declarator or expression), not type names like `void` in `main(void)`.
fn ensure_no_keyword_as_identifier(db: &dyn Db, file: SourceFile, root: TsNode, src: &str) {
    let lang = Language::new(tree_sitter_c::LANGUAGE);
    let query = tree_sitter::Query::new(&lang, "(identifier) @id").unwrap();
    let mut cursor = tree_sitter::QueryCursor::new();
    let mut matches = cursor.matches(&query, root, src.as_bytes());

    while let Some(m) = matches.next() {
        let node = m.captures[0].node;
        if is_type_only_identifier(node) {
            continue;
        }
        let text = node.utf8_text(src.as_bytes()).unwrap();
        if C_KEYWORDS.contains(&text) {
            Diagnostic::error()
                .with_message(format!(
                    "\"{text}\" is a keyword and cannot be used as an identifier"
                ))
                .with_code(codes::type_check::keyword_as_identifier)
                .with_labels(vec![
                    Label::primary(file, Span::for_node(node))
                        .with_message("keyword used as identifier"),
                ])
                .accumulate(db);
        }
    }
}

/// Emit invalid-type diagnostic for declarations in a function body that use type_identifier (e.g. "ints a").
fn ensure_valid_declaration_types<'db>(
    db: &'db dyn Db,
    file: SourceFile,
    body: ast::CompoundStatement<'db>,
) {
    let src = file.contents(db);
    let mut cursor = body.walk();
    for child in body.children(&mut cursor).filter_map(|c| c.ok()) {
        if let Some(decl) = child.as_declaration() {
            let raw = decl.raw();
            if let Some(type_spec) = raw.child_by_field_name("type") {
                if type_spec.kind() == "type_identifier" {
                    let text = type_spec
                        .utf8_text(src.as_bytes())
                        .unwrap_or_default()
                        .trim();
                    Diagnostic::error()
                        .with_message(format!("invalid type \"{text}\""))
                        .with_code(codes::type_check::invalid_type)
                        .with_labels(vec![
                            Label::primary(file, Span::for_node(type_spec))
                                .with_message("invalid or unsupported type"),
                        ])
                        .accumulate(db);
                }
            }
        }
    }
}

/// True if this identifier is only used as a type (e.g. `void` in `main(void)`).
fn is_type_only_identifier(node: TsNode) -> bool {
    let mut cur = node;
    while let Some(p) = cur.parent() {
        if ast::Declarator::try_from_raw(p).is_ok()
            || ast::InitDeclarator::try_from_raw(p).is_ok()
            || ast::FunctionDeclarator::try_from_raw(p).is_ok()
        {
            return false;
        }
        if ast::TypeSpecifier::try_from_raw(p).is_ok()
            || ast::ParameterDeclaration::try_from_raw(p).is_ok()
            || ast::ParameterList::try_from_raw(p).is_ok()
        {
            return true;
        }
        cur = p;
    }
    false
}

/// A typechecking context - roughly equal to a "scope".
#[salsa::tracked(debug)]
pub struct TyCtx<'db> {
    #[returns(ref)]
    pub defs: Vec<hir::Def<'db>>,
    pub parent: Option<TyCtx<'db>>,
}

#[salsa::tracked]
impl<'db> TyCtx<'db> {
    pub fn lookup(self, db: &'db dyn Db, name: hir::Identifier<'db>) -> Option<hir::Def<'db>> {
        let names = self.names(db);
        names.get(&name).copied()
    }

    pub fn names(&self, db: &'db dyn Db) -> im::OrdMap<hir::Identifier<'db>, hir::Def<'db>> {
        let mut names = self.parent(db).map(|p| p.names(db)).unwrap_or_default();

        for &def in self.defs(db) {
            names.insert(def.name(db), def);
        }

        names
    }
}

#[cfg(test)]
mod tests {
    use crate::{Database, diagnostics::Diagnostics};

    use super::*;

    #[test]
    fn test_function_definition() {
        let db = Database::default();
        let src = "int main(void) {}";
        let file = SourceFile::new(&db, "test.c".into(), src.into());
        let tu = typecheck(&db, file);

        assert_eq!(tu.items(&db).len(), 1);
        assert_eq!(tu.items(&db)[0].name(&db).text(&db), "main");
    }

    #[test]
    fn function_signature_returns_type_and_parameters() {
        let db = Database::default();
        let src = "int main(void) { return 0; }";
        let file = SourceFile::new(&db, "test.c".into(), src.into());
        let tu = typecheck(&db, file);

        let item = &tu.items(&db)[0];
        let hir::Item::Function(f) = item;
        let sig = function_signature(&db, *f);

        assert_eq!(sig.return_type, hir::Type::Int);
        assert!(sig.parameters.is_empty());
    }

    #[test]
    fn typecheck_diagnoses_missing_return_type() {
        let db = Database::default();
        // Old-style definition with no return type; we report at typecheck (missing return type and/or keyword).
        let src = "main(void) { return 0; }";
        let file = SourceFile::new(&db, "test.c".into(), src.into());
        let _ = typecheck(&db, file);
        let diags = typecheck::accumulated::<Diagnostics>(&db, file);
        assert!(
            !diags.is_empty(),
            "expected at least one typecheck diagnostic for invalid program, got none"
        );
        assert!(
            diags.iter().any(|d| {
                d.message == "Expected a return type for function"
                    || (d.message.contains("keyword") && d.message.contains("identifier"))
            }),
            "expected missing return type or keyword diagnostic, got {diags:?}"
        );
    }

    #[test]
    fn typecheck_diagnoses_keyword_as_identifier() {
        let db = Database::default();
        let src = "int main(void) { int return = 4; return return; }";
        let file = SourceFile::new(&db, "test.c".into(), src.into());
        let _ = typecheck(&db, file);
        let diags = typecheck::accumulated::<Diagnostics>(&db, file);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("keyword") && d.message.contains("identifier")),
            "expected keyword-as-identifier diagnostic, got {diags:?}"
        );
    }

    #[test]
    fn typecheck_diagnoses_invalid_type_specifier() {
        let db = Database::default();
        let src = "ints main(void) { return 0; }";
        let file = SourceFile::new(&db, "test.c".into(), src.into());
        let _ = typecheck(&db, file);
        let diags = typecheck::accumulated::<Diagnostics>(&db, file);
        assert!(
            diags.iter().any(|d| d.message.contains("invalid type")),
            "expected invalid type diagnostic, got {diags:?}"
        );
    }
}
