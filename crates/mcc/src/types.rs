use std::{
    fmt::{self, Display},
    ops::Deref,
};

use mcc_syntax::ast::TranslationUnit;
use type_sitter::Node;

use crate::{Db, Text};

#[derive(mcc_macros::SerializeWithDatabase)]
#[salsa::input]
#[derive(Debug)]
pub struct SourceFile {
    #[returns(ref)]
    pub path: Text,
    #[returns(ref)]
    pub contents: Text,
}

#[salsa::tracked]
pub struct Ast<'db> {
    #[returns(ref)]
    pub tree: Tree,
}

#[salsa::tracked]
impl<'db> Ast<'db> {
    pub fn sexpr(&self, db: &'db dyn Db) -> impl Display {
        SexpPrinter::new(self.tree(db).root_node())
    }

    pub fn root(&self, db: &'db dyn Db) -> TranslationUnit<'db> {
        let root = self.tree(db).root_node();
        TranslationUnit::try_from_raw(root).unwrap()
    }
}

#[derive(Clone)]
struct SexpPrinter<'db> {
    node: tree_sitter::Node<'db>,
    indent: usize,
}

impl<'db> SexpPrinter<'db> {
    fn new(node: tree_sitter::Node<'db>) -> Self {
        Self { node, indent: 0 }
    }
}

impl<'db> Display for SexpPrinter<'db> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn write_indent(f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
            for _ in 0..indent {
                write!(f, "  ")?;
            }
            Ok(())
        }

        fn escape_quoted_atom(s: &str) -> String {
            let mut out = String::with_capacity(s.len() + 4);
            for ch in s.chars() {
                match ch {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    _ => out.push(ch),
                }
            }
            out
        }

        fn write_node(
            f: &mut fmt::Formatter<'_>,
            node: tree_sitter::Node<'_>,
            indent: usize,
            emit_indent: bool,
        ) -> fmt::Result {
            if emit_indent {
                write_indent(f, indent)?;
            }

            // Handle missing nodes specially to match tree-sitter's S-expression output
            if node.is_missing() {
                let kind = node.kind();
                let quoted = escape_quoted_atom(kind);
                write!(f, "(MISSING \"{quoted}\")")?;
                return Ok(());
            }

            let is_error = node.is_error();
            if is_error {
                write!(f, "(ERROR")?;
            } else {
                let k = node.kind();
                if k.contains(|c: char| !c.is_alphanumeric() && c != '_') {
                    write!(f, "(\"{k}\"")?;
                } else {
                    write!(f, "({k}")?;
                }
            }

            let child_count = node.child_count();
            if child_count > 0 {
                // Each child on its own line, indented one level deeper
                for i in 0..child_count {
                    // Safety: index is in-bounds per child_count
                    if let Some(child) = node.child(i) {
                        writeln!(f)?;
                        write_indent(f, indent + 1)?;

                        if let Some(field_name) = node.field_name_for_child(i as u32) {
                            write!(f, "{field_name}: ")?;
                            // Child already has indentation emitted above; don't emit again here
                            write_node(f, child, indent + 1, false)?;
                        } else {
                            write_node(f, child, indent + 1, false)?;
                        }
                    }
                }
            }

            write!(f, ")")
        }

        write_node(f, self.node, self.indent, true)
    }
}

#[derive(Debug, Clone)]
pub struct Tree(pub tree_sitter::Tree);

impl From<tree_sitter::Tree> for Tree {
    fn from(value: tree_sitter::Tree) -> Self {
        Tree(value)
    }
}

impl Deref for Tree {
    type Target = tree_sitter::Tree;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq for Tree {
    fn eq(&self, other: &Self) -> bool {
        self.0.root_node() == other.0.root_node()
    }
}

impl Eq for Tree {}

impl std::hash::Hash for Tree {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.root_node().hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! sexpr_test {
        (
            $(
                $(#[$meta:meta])*
                $name:ident : $sexpr:expr
            ),*
            $(,)?
        ) => {
            $(
                $(#[$meta])*
                #[test]
                fn $name() {
                    let mut parser = tree_sitter::Parser::new();
                    parser.set_language(&tree_sitter_c::LANGUAGE.into()).unwrap();
                    let tree = parser.parse($sexpr, None).unwrap();
                    let root = tree.root_node();

                    println!("{}", root.to_sexp());
                    insta::with_settings!(
                        { description => $sexpr },
                        {
                            let formatted = SexpPrinter::new(root).to_string();
                            insta::assert_snapshot!(formatted);
                        }
                    );
                }
            )*
        };
    }

    sexpr_test! {
        empty: "",
        comment: "/* */",
        declaration: "int x;",
        function_definition: "void main();",
        function_call: "void main(void)",
        unclosed_paren: "int (",
    }
}
