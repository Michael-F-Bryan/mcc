use codespan::{ByteIndex, ByteOffset, ByteSpan, FileMap};
use codespan_reporting::{Diagnostic, Label};
use crate::ast::File;
use crate::grammar::{FileParser, Token};
use crate::node_id;
use lalrpop_util::ParseError;

/// Parse the contents of a file into its *Abstract Syntax Tree*
/// representation.
pub fn parse(filemap: &FileMap) -> Result<File, Diagnostic> {
    let base_offset = filemap.span().start() - ByteIndex(0);

    let mut parsed = FileParser::new()
        .parse(filemap.src())
        .map_err(|e| e.map_location(|l| ByteIndex(l as u32) - base_offset))
        .map_err(|e| translate_parse_error(filemap, e))?;

    fix_up(&mut parsed, base_offset);

    Ok(parsed)
}

fn translate_parse_error(
    filemap: &FileMap,
    err: ParseError<ByteIndex, Token<'_>, &str>,
) -> Diagnostic {
    let base_offset = filemap.span().start() - ByteIndex(0);

    match err {
        ParseError::InvalidToken { location } => {
            let loc = location - base_offset;
            let span = ByteSpan::new(loc, loc + ByteOffset(1));

            if filemap.span().contains(span) {
                Diagnostic::new_error("Invalid Token").with_label(Label::new_primary(span))
            } else {
                Diagnostic::new_error("Unexpected end of input")
            }
        }
        ParseError::UnrecognizedToken {
            token: None,
            expected,
        } => {
            let msg = if expected.is_empty() {
                "Unrecognised token".to_string()
            } else if expected.len() == 1 {
                format!("Expected {}", expected[0])
            } else {
                format!("Expected one of {}", expected.join("or"))
            };

            Diagnostic::new_error(msg)
        }
        ParseError::UnrecognizedToken {
            token: Some((start, tok, end)),
            expected,
        } => {
            let span = ByteSpan::new(start, end);
            let mut label = Label::new_primary(span);

            if expected.len() == 1 {
                label = label.with_message(format!("Expected {}", expected[0]));
            } else if expected.len() > 1 {
                label = label.with_message(format!("Expected one of {}", expected.join("or")));
            }

            Diagnostic::new_error(format!("Unrecognised token, {}", tok)).with_label(label)
        }
        ParseError::ExtraToken {
            token: (start, tok, end),
        } => {
            let span = ByteSpan::new(start, end);
            Diagnostic::new_error("Extra token").with_label(Label::new_primary(span))
        }
        ParseError::User { error } => Diagnostic::new_error(error),
    }
}

fn fix_up(file: &mut File, _base_offset: ByteOffset) {
    // TODO: Update all span locations
    node_id::assign_node_ids(file);
}

pub(crate) fn bs(left: usize, right: usize) -> ByteSpan {
    ByteSpan::new(ByteIndex(left as u32), ByteIndex(right as u32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{
        Expression, FnDecl, Function, Ident, Item, Literal, LiteralKind, Return, Statement, Type,
    };
    use crate::grammar::{FnDeclParser, ItemParser, LiteralParser, StatementParser};

    #[test]
    fn parse_a_literal() {
        let src = "123";
        let should_be = Literal::new(123.into(), bs(0, 3));

        let got = LiteralParser::new().parse(src).unwrap();

        assert_eq!(got, should_be);
    }

    #[test]
    fn parse_a_bare_return() {
        let src = "return;";
        let should_be = Statement::from(Return::bare(bs(0, src.len())));

        let got = StatementParser::new().parse(src).unwrap();

        assert_eq!(got, should_be);
    }

    #[test]
    fn parse_a_return_with_value() {
        let src = "return 5;";

        let five = Expression::Literal(Literal::new(LiteralKind::Integer(5), bs(7, 8)));
        let ret = Return::value(five, bs(0, src.len()));
        let should_be = Statement::from(ret);

        let got = StatementParser::new().parse(src).unwrap();

        assert_eq!(got, should_be);
    }

    #[test]
    fn parse_a_simple_function_signature() {
        let src = "int main()";

        let int = Type::from(Ident::new("int", bs(0, 3)));
        let main = Ident::new("main", bs(4, 8));
        let should_be = FnDecl::new(main, int, Vec::new(), bs(0, src.len()));

        let got = FnDeclParser::new().parse(src).unwrap();

        assert_eq!(got, should_be);
    }

    #[test]
    fn parse_int_main_void() {
        let src = r#"
            int main() {
                return 5;
            }
        "#;

        let int = Type::from(Ident::new("int", bs(13, 16)));
        let main = Ident::new("main", bs(17, 21));
        let decl = FnDecl::new(main, int, Vec::new(), bs(13, 23));

        let five = Expression::Literal(Literal::new(LiteralKind::Integer(5), bs(49, 50)));
        let ret = Return::value(five, bs(42, 51));
        let body = vec![Statement::from(ret)];

        let should_be = Function::new(decl, body, bs(13, 65));
        let should_be = Item::from(should_be);

        let got = ItemParser::new().parse(src).unwrap();

        assert_eq!(got, should_be);
    }
}
