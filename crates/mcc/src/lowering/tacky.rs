//! Three Address Code intermediate representation.
//!
//! You'll probably want to check out the [`Program`] type first.

use mcc_syntax::Span;

use crate::Text;

#[derive(mcc_macros::SerializeWithDatabase)]
#[salsa::tracked]
pub struct Program<'db> {
    pub functions: Vec<FunctionDefinition<'db>>,
}

#[derive(mcc_macros::SerializeWithDatabase)]
#[salsa::tracked]
pub struct FunctionDefinition<'db> {
    pub name: Text,
    pub instructions: Vec<Instruction>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Instruction {
    Return(Val),
    Unary {
        op: UnaryOperator,
        src: Val,
        dst: Val,
    },
    Binary {
        op: BinaryOperator,
        left_src: Val,
        right_src: Val,
        dst: Val,
    },
    Copy {
        src: Val,
        dst: Val,
    },
    Jump {
        target: Text,
    },
    JumpIfZero {
        condition: Val,
        target: Text,
    },
    JumpIfNotZero {
        condition: Val,
        target: Text,
    },
    Label(Text),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum UnaryOperator {
    Complement,
    Negate,
    Not,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    LeftShift,
    RightShift,
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Val {
    Constant(i32),
    Var(Variable),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Variable {
    Named(Text),
    Anonymous(u32),
}
