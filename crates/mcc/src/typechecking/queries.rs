//! Salsa-tracked queries over HIR types.
//!
//! Lazy computation of derived HIR properties lives here rather than as methods on the HIR types
//! in [`hir`], so the HIR module stays a pure data representation.

use type_sitter::Node;

use crate::Db;

use super::hir;

/// Lazily compute a function's signature (return type and parameters) from the AST.
#[salsa::tracked]
pub fn function_signature<'db>(
    db: &'db dyn Db,
    f: hir::FunctionDefinition<'db>,
) -> hir::Signature<'db> {
    let node = f.node(db).node(db);
    let file = f.node(db).file(db);
    let src = file.contents(db);
    let raw = node.raw();

    let return_type = raw
        .child_by_field_name("declaration_specifiers")
        .and_then(|spec| spec.utf8_text(src.as_bytes()).ok())
        .map(|s| match s.trim() {
            "int" => hir::Type::Int,
            "void" => hir::Type::Void,
            _ => hir::Type::Error,
        })
        .unwrap_or(hir::Type::Error);

    let parameters = Vec::new();

    hir::Signature {
        return_type,
        parameters,
    }
}
