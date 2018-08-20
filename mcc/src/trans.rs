//! Translation from AST to HIR.

use codespan::ByteSpan;
use codespan_reporting::{Diagnostic, Label};
use crate::hir::{CompilationUnit, Function, HirId, HirIdGenerator};
use crate::Diagnostics;
use heapsize_derive::HeapSizeOf;
use syntax::ast::{self, AstNode, File};
use syntax::visitor::{self, Visitor};

/// Translate from the AST to a more compiler-friendly form.
pub fn translate(ast: &File, diagnostics: &mut Diagnostics) -> CompilationUnit {
    let mut trans = Translator::new(diagnostics);
    trans.visit_file(ast);
    trans.comp
}

#[derive(Debug, HeapSizeOf)]
struct Translator<'diag> {
    diags: &'diag mut Diagnostics,
    comp: CompilationUnit,
    hir_ids: HirIdGenerator,
}

impl<'diag> Translator<'diag> {
    pub fn new(diagnostics: &'diag mut Diagnostics) -> Self {
        Translator {
            diags: diagnostics,
            comp: CompilationUnit::new(),
            hir_ids: HirIdGenerator::new(),
        }
    }

    fn duplicate_name(&mut self, name: &str, span: ByteSpan) {
        let diag = Diagnostic::new_error("Name defined multiple times").with_label(
            Label::new_primary(span).with_message(format!("\"{}\" is already defined", name)),
        );
        self.diags.add(diag);
    }
}

impl<'diag> Visitor for Translator<'diag> {
    fn visit_function(&mut self, func: &ast::Function) {
        if self.comp.namespace.contains_key(func.name()) {
            self.duplicate_name(func.name(), func.span());
            return;
        }

        let hir_func = Function {
            node_id: self.hir_ids.next_id(),
            name: func.name().to_string(),
        };

        self.comp.add_function(func.node_id(), hir_func);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codespan::{FileMap, FileName};

    fn int_main_void() -> FileMap {
        let src = "int main() { return 42; }";
        FileMap::new(FileName::virtual_("int_main_void"), src.to_string())
    }

    #[test]
    fn translate_int_main_void() {
        let fm = int_main_void();
        let ast = syntax::parse(&fm).unwrap();
        let mut diags = Diagnostics::new();

        let got = translate(&ast, &mut diags);

        assert!(diags.diagnostics().is_empty());
        assert_eq!(got.functions.len(), 1);

        let main_id = got.lookup("main").unwrap();
        let main = &got.functions[&main_id];
        assert_eq!(main.name, "main");

        let func = &ast.items[0];
        assert_eq!(got.node_id_mapping[&func.node_id()], main_id);
    }
}
