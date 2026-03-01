use codespan_reporting::diagnostic::Label;
use mcc_syntax::{Span, ast};
use type_sitter::{HasChildren, Node};

use crate::{
    Db, SourceFile, codes,
    diagnostics::{Diagnostic, DiagnosticExt, Diagnostics},
    ptr,
};

pub mod hir;
pub mod queries;

#[salsa::tracked]
#[tracing::instrument(level = "info", skip_all)]
pub fn typecheck<'db>(db: &'db dyn Db, file: SourceFile) -> hir::TranslationUnit<'db> {
    let ast = crate::parse(db, file);
    if !crate::parse::accumulated::<Diagnostics>(db, file).is_empty() {
        return hir::TranslationUnit::new(db, Vec::new(), file);
    }

    let tu = ast.root(db);
    let mut cursor = tu.walk();
    let mut items = Vec::new();

    for child in tu.children(&mut cursor).filter_map(|c| c.ok()) {
        type Child<'db> = <ast::TranslationUnit<'db> as HasChildren<'db>>::Child;
        match child {
            Child::FunctionDefinition(f) => {
                if let Some(f) = function_definition(db, file, f) {
                    items.push(hir::Item::Function(f));
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
    use crate::Database;

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
        let sig = queries::function_signature(&db, *f);

        assert_eq!(sig.return_type, hir::Type::Int);
        assert!(sig.parameters.is_empty());
    }
}
