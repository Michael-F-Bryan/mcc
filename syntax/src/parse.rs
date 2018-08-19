use codespan::{ByteIndex, ByteSpan, FileMap};
use crate::ast::File;
use crate::grammar;

pub fn parse(file: &FileMap) -> Result<File, ()> {
    unimplemented!()
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
