//! Assembly instructions.

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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Instruction {
    Mov {
        src: Operand,
        dst: Operand,
    },
    Unary {
        op: UnaryOperator,
        operand: Operand,
    },
    Binary {
        op: BinaryOperator,
        src: Operand,
        dst: Operand,
    },
    /// Divide `EAX` by `src`, storing the quotient in `EAX` and the remainder
    /// in `EDX`.
    Idiv {
        src: Operand,
    },
    /// Sign-extend the value from `EAX` into `EDX`.
    Cdq,
    AllocateStack(u32),
    Ret,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Operand {
    Imm(i32),
    Register(Register),
    Stack(u32),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum UnaryOperator {
    Neg,
    Not,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Register {
    AX,
    DX,
    R10,
}
