//! Assembly instructions.

use mcc_syntax::Span;

use crate::Text;

#[derive(mcc_macros::SerializeWithDatabase)]
#[salsa::tracked]
#[derive(Debug)]
pub struct Program<'db> {
    pub functions: Vec<FunctionDefinition<'db>>,
}

#[derive(mcc_macros::SerializeWithDatabase)]
#[salsa::tracked]
#[derive(Debug)]
pub struct FunctionDefinition<'db> {
    pub name: Text,
    pub instructions: Vec<Instruction>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Instruction {
    /// Move a value from one operand to another.
    Mov { src: Operand, dst: Operand },
    /// Apply a unary operator to an operand.
    Unary { op: UnaryOperator, operand: Operand },
    /// Apply a binary operator to two operands.
    Binary {
        op: BinaryOperator,
        src: Operand,
        dst: Operand,
    },
    /// Divide `EAX` by `src`, storing the quotient in `EAX` and the remainder
    /// in `EDX`.
    Idiv { src: Operand },
    /// Sign-extend the value from `EAX` into `EDX`.
    Cdq,
    /// Allocate `n` bytes on the stack.
    AllocateStack(u32),
    /// Return from the current function.
    Ret,
    /// A label.
    Label(Text),
    /// Jump to a label.
    Jump { target: Text },
    /// Jump to a label if the condition is zero.
    JumpIfZero { condition: Operand, target: Text },
    /// Jump to a label if the condition is not zero.
    JumpIfNotZero { condition: Operand, target: Text },
    /// Compare two operands and set flags.
    Comparison {
        op: ComparisonOperator,
        left: Operand,
        right: Operand,
        dst: Operand,
    },
}

/// An operand is a value that can be used in an instruction.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Operand {
    /// A constant value.
    Imm(i32),
    /// A named register.
    Register(Register),
    /// Somewhere on the stack, as a byte offset from `%rbp`.
    Stack(u32),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum UnaryOperator {
    Neg,
    Complement,
    Not,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    And,
    Or,
    LeftShift,
    RightShift,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Register {
    AX,
    DX,
    R10,
}
