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
            tacky::Instruction::Binary {
                op,
                left_src,
                right_src,
                dst,
            } => {
                let left_src = stack_locations.operand_for(left_src);
                let right_src = stack_locations.operand_for(right_src);
                let dst = stack_locations.operand_for(dst);

                enum BinOpKind {
                    Bin(asm::BinaryOperator),
                    Div,
                    Mod,
                }

                let op = match op {
                    tacky::BinaryOperator::Add => BinOpKind::Bin(asm::BinaryOperator::Add),
                    tacky::BinaryOperator::Sub => BinOpKind::Bin(asm::BinaryOperator::Sub),
                    tacky::BinaryOperator::Mul => BinOpKind::Bin(asm::BinaryOperator::Mul),
                    tacky::BinaryOperator::And => BinOpKind::Bin(asm::BinaryOperator::And),
                    tacky::BinaryOperator::Or => BinOpKind::Bin(asm::BinaryOperator::Or),
                    tacky::BinaryOperator::LeftShift => {
                        BinOpKind::Bin(asm::BinaryOperator::LeftShift)
                    }
                    tacky::BinaryOperator::RightShift => {
                        BinOpKind::Bin(asm::BinaryOperator::RightShift)
                    }
                    tacky::BinaryOperator::Div => BinOpKind::Div,
                    tacky::BinaryOperator::Mod => BinOpKind::Mod,
                };

                match op {
                    BinOpKind::Bin(op) => {
                        instructions.push(asm::Instruction::Mov {
                            src: left_src,
                            dst: asm::Operand::Register(asm::Register::R10),
                        });
                        instructions.push(asm::Instruction::Binary {
                            op,
                            src: right_src,
                            dst: asm::Operand::Register(asm::Register::R10),
                        });
                        instructions.push(asm::Instruction::Mov {
                            src: asm::Operand::Register(asm::Register::R10),
                            dst,
                        });
                    }
                    BinOpKind::Div => {
                        instructions.push(asm::Instruction::Mov {
                            src: left_src,
                            dst: asm::Operand::Register(asm::Register::AX),
                        });
                        instructions.push(asm::Instruction::Cdq);
                        instructions.push(asm::Instruction::Idiv { src: right_src });
                        instructions.push(asm::Instruction::Mov {
                            src: asm::Operand::Register(asm::Register::AX),
                            dst,
                        });
                    }
                    BinOpKind::Mod => {
                        instructions.push(asm::Instruction::Mov {
                            src: left_src,
                            dst: asm::Operand::Register(asm::Register::AX),
                        });
                        instructions.push(asm::Instruction::Cdq);
                        instructions.push(asm::Instruction::Idiv { src: right_src });
                        instructions.push(asm::Instruction::Mov {
                            src: asm::Operand::Register(asm::Register::DX),
                            dst,
                        });
                    }
                }
            }
            tacky::Instruction::Comparison {
                op,
                left_src,
                right_src,
                dst,
            } => {
                let left_src = stack_locations.operand_for(left_src);
                let right_src = stack_locations.operand_for(right_src);
                let dst = stack_locations.operand_for(dst);

                let comparison_op = match op {
                    tacky::ComparisonOperator::Equal => asm::ComparisonOperator::Equal,
                    tacky::ComparisonOperator::NotEqual => asm::ComparisonOperator::NotEqual,
                    tacky::ComparisonOperator::LessThan => asm::ComparisonOperator::LessThan,
                    tacky::ComparisonOperator::LessThanOrEqual => {
                        asm::ComparisonOperator::LessThanOrEqual
                    }
                    tacky::ComparisonOperator::GreaterThan => asm::ComparisonOperator::GreaterThan,
                    tacky::ComparisonOperator::GreaterThanOrEqual => {
                        asm::ComparisonOperator::GreaterThanOrEqual
                    }
                };

                instructions.push(asm::Instruction::Comparison {
                    op: comparison_op,
                    left: left_src,
                    right: right_src,
                    dst,
                });
            }
            tacky::Instruction::Copy { src, dst } => {
                let src = stack_locations.operand_for(src);
                let dst = stack_locations.operand_for(dst);

                instructions.push(asm::Instruction::Mov { src, dst });
            }
            tacky::Instruction::Jump { target } => {
                instructions.push(asm::Instruction::Jump { target });
            }
            tacky::Instruction::Label(target) => {
                instructions.push(asm::Instruction::Label(target));
            }
            tacky::Instruction::JumpIfZero { condition, target } => {
                let condition = stack_locations.operand_for(condition);
                instructions.push(asm::Instruction::JumpIfZero { condition, target });
            }
            tacky::Instruction::JumpIfNotZero { condition, target } => {
                let condition = stack_locations.operand_for(condition);
                instructions.push(asm::Instruction::JumpIfNotZero { condition, target });
            }
        }
    }

    // Allocate stack space for local variables if needed. Each slot is 4 bytes.
    let stack_size_bytes = (stack_locations.variables.len() as u32) * 4;
    if stack_size_bytes > 0 {
        instructions.insert(0, asm::Instruction::AllocateStack(stack_size_bytes));
    }

    asm::FunctionDefinition::new(db, name, instructions, function.span(db))
}

fn unary_operator_to_asm(op: tacky::UnaryOperator) -> asm::UnaryOperator {
    match op {
        tacky::UnaryOperator::Negate => asm::UnaryOperator::Neg,
        tacky::UnaryOperator::Complement => asm::UnaryOperator::Complement,
        tacky::UnaryOperator::Not => asm::UnaryOperator::Not,
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

/// Fixes up invalid `mov` instructions where both source and destination are
/// stack operands.
///
/// When replacing pseudoregisters with stack addresses, we may end up with
/// `mov` instructions that have memory addresses as both source and destination
/// operands. This is invalid assembly, as instructions like `movl -4(%rbp),
/// -8(%rbp)` will be rejected by the assembler.
///
/// The fix is to rewrite such instructions to:
/// 1. First copy from the source stack location into R10D
/// 2. Then copy from R10D to the destination stack location
#[tracing::instrument(level = "debug", skip_all)]
#[salsa::tracked]
fn fix_up_instructions<'db>(
    db: &'db dyn Db,
    function: asm::FunctionDefinition<'db>,
) -> asm::FunctionDefinition<'db> {
    let mut instructions = Vec::new();

    for instruction in function.instructions(db) {
        match instruction {
            asm::Instruction::Mov {
                src: src @ asm::Operand::Stack(_),
                dst: dst @ asm::Operand::Stack(_),
            } => {
                // `mov` instructions with memory addresses as both source and
                // destination are invalid assembly, so we need to move the
                // source to a register first.
                instructions.push(asm::Instruction::Mov {
                    src,
                    dst: asm::Operand::Register(asm::Register::R10),
                });
                instructions.push(asm::Instruction::Mov {
                    src: asm::Operand::Register(asm::Register::R10),
                    dst,
                });
            }
            asm::Instruction::Idiv {
                src: src @ asm::Operand::Imm(_),
            } => {
                // `idiv` does not accept an immediate, so we need to move the
                // source to a register first.
                instructions.push(asm::Instruction::Mov {
                    src,
                    dst: asm::Operand::Register(asm::Register::R10),
                });
                instructions.push(asm::Instruction::Idiv {
                    src: asm::Operand::Register(asm::Register::R10),
                });
            }
            asm::Instruction::Comparison {
                op,
                left: left @ asm::Operand::Imm(_),
                right: right @ asm::Operand::Imm(_),
                dst,
            } => {
                // `cmpl` does not accept two immediates, so we need to move one
                // to a register first.
                instructions.push(asm::Instruction::Mov {
                    src: left,
                    dst: asm::Operand::Register(asm::Register::R10),
                });
                instructions.push(asm::Instruction::Comparison {
                    op,
                    left: asm::Operand::Register(asm::Register::R10),
                    right,
                    dst,
                });
            }
            asm::Instruction::Comparison {
                op,
                left: left @ asm::Operand::Stack(_),
                right: right @ asm::Operand::Imm(_),
                dst,
            } => {
                // `cmpl` does not accept memory as destination with immediate source,
                // so we need to move the memory operand to a register first.
                instructions.push(asm::Instruction::Mov {
                    src: left,
                    dst: asm::Operand::Register(asm::Register::R10),
                });
                instructions.push(asm::Instruction::Comparison {
                    op,
                    left: asm::Operand::Register(asm::Register::R10),
                    right,
                    dst,
                });
            }
            asm::Instruction::Comparison {
                op,
                left: left @ asm::Operand::Imm(_),
                right: right @ asm::Operand::Stack(_),
                dst,
            } => {
                // `cmpl` does not accept memory as destination with immediate source,
                // so we need to move the memory operand to a register first.
                instructions.push(asm::Instruction::Mov {
                    src: right,
                    dst: asm::Operand::Register(asm::Register::R10),
                });
                instructions.push(asm::Instruction::Comparison {
                    op,
                    left: asm::Operand::Register(asm::Register::R10),
                    right: left,
                    dst,
                });
            }
            other => instructions.push(other),
        }
    }

    asm::FunctionDefinition::new(db, function.name(db), instructions, function.span(db))
}

#[cfg(test)]
mod tests {
    use crate::{SerializeWithDatabase, types::SourceFile};

    #[test]
    fn simplest_program() {
        let src = "int main(void) { return 0; }";
        let db = salsa::DatabaseImpl::default();
        let file = SourceFile::new(&db, "main.c".into(), src.into());
        let ast = crate::parse(&db, file);
        let tacky = crate::lower(&db, ast, file);
        let assembly = crate::generate_assembly(&db, tacky);

        insta::assert_json_snapshot!(assembly.serialize_with_db(&db));
    }
}
