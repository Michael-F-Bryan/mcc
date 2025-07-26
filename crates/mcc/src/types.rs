use std::ops::Deref;

use crate::{Db, Text};

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
    pub tree: Tree,
}

#[salsa::tracked]
impl<'db> Ast<'db> {
    pub fn sexpr(&self, db: &'db dyn Db) -> String {
        let raw = self.tree(db).root_node().to_sexp();
        format_sexpr(&raw)
    }
}

/// A quick'n'dirty s-expression pretty-printer.
fn format_sexpr(raw: &str) -> String {
    let mut result = String::new();
    let mut depth = 0;
    let mut in_word = false;
    let mut after_colon = false;
    let mut field_start = 0;

    for (i, c) in raw.chars().enumerate() {
        match c {
            '(' => {
                if in_word {
                    result.push(' ');
                    in_word = false;
                }
                if !after_colon {
                    result.push('\n');
                    result.extend(std::iter::repeat("  ").take(depth));
                } else {
                    // After a field, indent to align with the content
                    let field_length = i - field_start;
                    result.push('\n');
                    result.extend(std::iter::repeat("  ").take(depth));
                    result.extend(std::iter::repeat(" ").take(field_length));
                }
                result.push('(');
                depth += 1;
                after_colon = false;
            }
            ')' => {
                depth -= 1;
                result.push(')');
                in_word = false;
                after_colon = false;
            }
            ' ' | '\n' => {
                if in_word {
                    result.push(' ');
                    in_word = false;
                }
            }
            ':' => {
                result.push(c);
                after_colon = true;
                in_word = true;
                field_start = i + 1; // Start of the field name
            }
            _ => {
                if !in_word && !result.ends_with('(') && !after_colon {
                    result.push('\n');
                    result.extend(std::iter::repeat("  ").take(depth));
                }
                result.push(c);
                in_word = true;
                if c != ' ' && c != '\n' {
                    after_colon = false;
                }
            }
        }
    }

    result
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    pub start: usize,
    pub length: usize,
}

impl Span {
    pub const fn new(start: usize, length: usize) -> Self {
        Span { start, length }
    }

    pub const fn end(&self) -> usize {
        self.start + self.length
    }

    pub fn for_node(node: tree_sitter::Node<'_>) -> Self {
        node.range().into()
    }

    pub const fn to_range(self) -> std::ops::Range<usize> {
        self.start..self.end()
    }

    pub fn lookup(self, text: &str) -> &str {
        &text[self.to_range()]
    }
}

impl From<tree_sitter::Range> for Span {
    fn from(range: tree_sitter::Range) -> Self {
        Span::new(range.start_byte, range.end_byte - range.start_byte)
    }
}

impl From<std::ops::Range<usize>> for Span {
    fn from(value: std::ops::Range<usize>) -> Self {
        Span::new(value.start, value.len())
    }
}

impl From<Span> for std::ops::Range<usize> {
    fn from(value: Span) -> Self {
        value.to_range()
    }
}
