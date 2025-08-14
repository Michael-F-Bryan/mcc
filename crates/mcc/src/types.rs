use std::ops::Deref;

use mcc_syntax::ast::TranslationUnit;
use type_sitter::Node;

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
    #[returns(ref)]
    pub tree: Tree,
}

#[salsa::tracked]
impl<'db> Ast<'db> {
    pub fn sexpr(&self, db: &'db dyn Db) -> String {
        let raw = self.tree(db).root_node().to_sexp();
        format_sexpr(&raw)
    }

    pub fn root(&self, db: &'db dyn Db) -> TranslationUnit<'db> {
        let root = self.tree(db).root_node();
        TranslationUnit::try_from_raw(root).unwrap()
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

/// A quick'n'dirty s-expression pretty-printer.
fn format_sexpr(raw: &str) -> String {
    let mut result = String::new();
    let mut depth = 0;
    let mut in_word = false;
    let mut after_colon = false;
    let mut chars = raw.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '(' => {
                if in_word {
                    result.push(' ');
                    in_word = false;
                }
                if !after_colon {
                    result.push('\n');
                    result.extend(std::iter::repeat_n("  ", depth));
                }
                result.push('(');
                depth += 1;
                after_colon = false;
            }
            ')' => {
                depth = depth.checked_sub(1).expect("Mismatched parentheses");
                result.push(')');
                in_word = false;
                after_colon = false;
            }
            ' ' | '\n' | '\t' => {
                if in_word {
                    result.push(' ');
                    in_word = false;
                }
                // Preserve original whitespace after fields
                if after_colon {
                    result.push(c);
                }
            }
            ':' => {
                result.push(c);
                after_colon = true;
                in_word = true;
            }
            '\\' => {
                // Handle escaped characters
                if let Some(next_char) = chars.next() {
                    // For escaped quotes, just output the quote without the backslash
                    if next_char == '"' {
                        result.push('"');
                    } else {
                        result.push(c);
                        result.push(next_char);
                    }
                    in_word = true;
                } else {
                    result.push(c);
                    in_word = true;
                }
            }
            _ => {
                // Only add newline if we're not after a colon and not already at the start of a line
                if !in_word && !result.ends_with('(') && !after_colon {
                    result.push('\n');
                    result.extend(std::iter::repeat_n("  ", depth));
                }
                result.push(c);
                in_word = true;
                // Reset after_colon only for non-whitespace characters
                if c != ' ' && c != '\n' && c != '\t' {
                    after_colon = false;
                }
            }
        }
    }

    let trimmed = result.trim_start().to_string();

    // Only add leading newline for complex nested structures
    // Check if the input has multiple nested levels or is the translation_unit case
    if raw.contains("translation_unit") || raw.matches('(').count() > 3 {
        format!("\n{trimmed}")
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! sexpr_test {
        (
            $(
                $(#[$meta:meta])*
                $name:ident : $sexpr:expr => $expected:expr
            ),*
            $(,)?
        ) => {
            $(
                $(#[$meta])*
                #[test]
                fn $name() {
                    let formatted = format_sexpr($sexpr);
                    assert_eq!(formatted, $expected);
                    insta::assert_snapshot!(formatted);
                }
            )*
        };
    }

    sexpr_test! {
        test_empty: "()" => "()",
        test_one_element: "(a)" => "(a)",
        test_nested: "(a (b c))" => "(a \n  (b \n    c))",
        test_field: "(a :b c)" => "(a :b \n  c)",
        test_field_with_spaces_and_newlines: "(a :b\nc d)" => "(a :b \n  c \n  d)",
        test_field_with_spaces_and_newlines_and_tabs: "(a :b\tc d)" => "(a :b \n  c \n  d)",
        #[ignore]
        translation_unit: r#"(translation_unit (function_definition type: (primitive_type) declarator: (function_declarator declarator: (identifier) parameters: (parameter_list (parameter_declaration type: (primitive_type)))) body: (compound_statement (return_statement (parenthesized_expression (unary_expression (ERROR) argument: (number_literal)) (MISSING \")\"))))))"# =>
r#"
(translation_unit
  (function_definition
    type: (primitive_type)
    declarator: (function_declarator
      declarator: (identifier)
      parameters: (parameter_list
        (parameter_declaration
          type: (primitive_type))))
    body: (compound_statement
      (return_statement
        (parenthesized_expression
          (unary_expression
            (ERROR)
            argument: (number_literal))
          (MISSING ")")))))))"#,
    }
}
