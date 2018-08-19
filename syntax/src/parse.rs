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
    use crate::ast::{Expression, Literal, LiteralKind, Return, Statement};
    use crate::grammar::{LiteralParser, StatementParser};

    #[test]
    fn parse_a_literal() {
        let src = "123";
        let should_be = Literal::new(123.into(), bs(0, 3));

        let got = LiteralParser::new().parse(src).unwrap();

        assert_eq!(got, should_be);
    }

    #[test]
    fn parse_a_bare_return() {
        let src = "return";
        let should_be = Statement::from(Return::bare(bs(0, src.len())));

        let got = StatementParser::new().parse(src).unwrap();

        assert_eq!(got, should_be);
    }

    #[test]
    fn parse_a_return_with_value() {
        let src = "return 5";

        let five = Expression::Literal(Literal::new(LiteralKind::Integer(5), bs(7, 8)));
        let ret = Return::value(five, bs(0, src.len()));
        let should_be = Statement::from(ret);

        let got = StatementParser::new().parse(src).unwrap();

        assert_eq!(got, should_be);
    }
}
