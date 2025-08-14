//! Compile [Three Address Code](crate::lowering::tacky) to [Assembly](asm).

pub mod asm;

use crate::{Db, lowering::tacky};

/// Compile a parsed C program into assembly.
#[tracing::instrument(level = "info", skip_all)]
#[salsa::tracked]
pub fn generate_assembly<'db>(db: &'db dyn Db, program: tacky::Program<'db>) -> asm::Program<'db> {
    let mut functions = Vec::new();
    for function in program.functions(db) {
        functions.push(lower_function(db, function));
    }

    asm::Program::new(db, functions)
}

/// Lower from [Three Address Code](crate::lowering::tacky) to [Assembly](asm).
#[tracing::instrument(level = "debug", skip_all, fields(name = %function.name(db)))]
#[salsa::tracked]
fn lower_function<'db>(
    db: &'db dyn Db,
    function: tacky::FunctionDefinition<'db>,
) -> asm::FunctionDefinition<'db> {
    let asm = to_assembly(db, function);
    fix_up_instructions(db, asm)
}

/// Lower a [`tacky::FunctionDefinition`] to [`asm::FunctionDefinition`],
/// placing variables and pseudo-registers on the stack.
#[tracing::instrument(level = "debug", skip_all)]
#[salsa::tracked]
fn to_assembly<'db>(
    db: &'db dyn Db,
    function: tacky::FunctionDefinition<'db>,
) -> asm::FunctionDefinition<'db> {
    let name = function.name(db);
    let mut instructions = Vec::new();
    let mut stack_locations = StackAllocator::default();

    for instruction in function.instructions(db) {
        match instruction {
            tacky::Instruction::Return(ret) => {
                let src = stack_locations.operand_for(ret);

                instructions.push(asm::Instruction::Mov {
                    src,
                    dst: asm::Operand::Register(asm::Register::AX),
                });
                instructions.push(asm::Instruction::Ret);
            }
            tacky::Instruction::Unary { op, src, dst } => {
                let op = unary_operator_to_asm(op);
                let src = stack_locations.operand_for(src);
                let dst = stack_locations.operand_for(dst);

                instructions.push(asm::Instruction::Mov { src, dst });
                instructions.push(asm::Instruction::Unary { op, operand: dst });
            }
        }
    }

    asm::FunctionDefinition::new(db, name, instructions, function.span(db))
}

fn unary_operator_to_asm(op: tacky::UnaryOperator) -> asm::UnaryOperator {
    match op {
        tacky::UnaryOperator::Negate => asm::UnaryOperator::Neg,
        tacky::UnaryOperator::Complement => asm::UnaryOperator::Not,
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
struct StackAllocator {
    variables: Vec<tacky::Variable>,
}

impl StackAllocator {
    fn operand_for(&mut self, val: tacky::Val) -> asm::Operand {
        match val {
            tacky::Val::Constant(c) => asm::Operand::Imm(c),
            tacky::Val::Var(v) => asm::Operand::Stack(self.offset_for(v)),
        }
    }

    fn offset_for(&mut self, variable: tacky::Variable) -> u32 {
        (self.index_of(variable) as u32) * 4
    }

    fn index_of(&mut self, variable: tacky::Variable) -> usize {
        match self.variables.iter().position(|v| v == &variable) {
            Some(i) => i,
            None => {
                let index = self.variables.len();
                self.variables.push(variable);
                index
            }
        }
    }
}

#[tracing::instrument(level = "debug", skip_all)]
#[salsa::tracked]
fn fix_up_instructions<'db>(
    _db: &'db dyn Db,
    function: asm::FunctionDefinition<'db>,
) -> asm::FunctionDefinition<'db> {
    function
}
