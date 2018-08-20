//! Translation from AST to HIR.

use crate::hir::CompilationUnit;
use crate::Diagnostics;
use heapsize_derive::HeapSizeOf;
use syntax::ast::File;

pub fn translate(ast: &File, diagnostics: &mut Diagnostics) -> CompilationUnit {
    let mut trans = Translator::new(diagnostics);
    trans.comp
}

#[derive(Debug, HeapSizeOf)]
struct Translator<'diag> {
    diags: &'diag mut Diagnostics,
    comp: CompilationUnit,
}

impl<'diag> Translator<'diag> {
    pub fn new(diagnostics: &'diag mut Diagnostics) -> Self {
        Translator {
            diags: diagnostics,
            comp: CompilationUnit::new(),
        }
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

        assert!(!diags.has_warnings());

        assert_eq!(got.functions.len(), 1);
        let main = got.functions.values().next().unwrap();
    }
}
