//! Three Address Code intermediate representation.

use mcc_syntax::Span;

use crate::Text;

#[salsa::tracked]
pub struct Program<'db> {
    pub functions: Vec<FunctionDefinition<'db>>,
}

#[salsa::tracked]
pub struct FunctionDefinition<'db> {
    pub name: Text,
    pub instructions: Vec<Instruction>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Instruction {
    Return(Val),
    Unary {
        op: UnaryOperator,
        src: Val,
        dst: Val,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UnaryOperator {
    Complement,
    Negate,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Val {
    Constant(i32),
    Var(Variable),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Variable {
    Named(Text),
    Anonymous(u32),
}
