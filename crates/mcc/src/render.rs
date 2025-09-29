use std::{
    borrow::Cow,
    fmt::{self, Write},
};

use target_lexicon::{OperatingSystem, Triple};

use crate::{Db, Text, codegen::asm};

/// Render a set of assembly instructions as a string.
#[tracing::instrument(level = "debug", skip_all, fields(target = %target))]
#[salsa::tracked]
pub fn render_program<'db>(
    db: &'db dyn Db,
    program: asm::Program<'db>,
    target: Triple,
) -> Result<Text, fmt::Error> {
    let mut output = String::new();
    let mut renderer = AssemblyRenderer::new(target, &mut output);
    renderer.program(db, program)?;
    Ok(output.into())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AssemblyRenderer<W> {
    target: Triple,
    writer: W,
}

impl<W: Write> AssemblyRenderer<W> {
    fn new(target: Triple, writer: W) -> Self {
        Self { target, writer }
    }

    fn program(&mut self, db: &dyn Db, program: asm::Program) -> fmt::Result {
        for function in program.functions(db) {
            self.render_function(db, function)?;
            writeln!(self.writer)?;
        }

        if self.target.operating_system == OperatingSystem::Linux {
            writeln!(self.writer, ".section .note.GNU-stack, \"\", @progbits")?;
        }

        Ok(())
    }

    fn function_name<'a>(&self, name: &'a str) -> Cow<'a, str> {
        if matches!(
            self.target.operating_system,
            OperatingSystem::MacOSX(_) | OperatingSystem::Darwin(_)
        ) {
            format!("_{name}").into()
        } else {
            name.into()
        }
    }

    pub fn render_function(
        &mut self,
        db: &dyn Db,
        function: asm::FunctionDefinition,
    ) -> fmt::Result {
        let name = function.name(db);
        let name = self.function_name(&name);

        writeln!(self.writer, ".globl {name}")?;
        writeln!(self.writer, "{name}:")?;
        writeln!(self.writer, "pushq %rbp")?;
        writeln!(self.writer, "movq %rsp, %rbp")?;

        for instruction in function.instructions(db) {
            write!(self.writer, "  ")?;
            self.render_instruction(instruction)?;
        }

        Ok(())
    }

    fn render_instruction(&mut self, instruction: asm::Instruction) -> fmt::Result {
        match instruction {
            asm::Instruction::AllocateStack(size) => {
                writeln!(self.writer, "subq ${size}, %rsp")?;
            }
            asm::Instruction::Mov { src, dst } => {
                write!(self.writer, "movl ")?;
                self.operand(src)?;
                write!(self.writer, ", ")?;
                self.operand(dst)?;
                writeln!(self.writer)?;
            }
            asm::Instruction::Unary { op, operand } => {
                match op {
                    asm::UnaryOperator::Not => {
                        // Logical NOT: compare with 0 and set result to 1 if zero, 0 if non-zero
                        write!(self.writer, "cmpl $0, ")?;
                        self.operand(operand)?;
                        writeln!(self.writer)?;
                        write!(self.writer, "sete %al")?;
                        writeln!(self.writer)?;
                        write!(self.writer, "movb %al, ")?;
                        self.operand(operand)?;
                        writeln!(self.writer)?;
                    }
                    _ => {
                        self.unary_operator(op)?;
                        write!(self.writer, " ")?;
                        self.operand(operand)?;
                        writeln!(self.writer)?;
                    }
                }
            }
            asm::Instruction::Ret => {
                writeln!(self.writer, "movq %rbp, %rsp")?;
                writeln!(self.writer, "popq %rbp")?;
                writeln!(self.writer, "ret")?;
            }
            asm::Instruction::Binary { op, src, dst } => {
                self.binary_operator(op)?;
                write!(self.writer, " ")?;
                self.operand(src)?;
                write!(self.writer, ", ")?;
                self.operand(dst)?;
                writeln!(self.writer)?;
            }
            asm::Instruction::Comparison {
                op,
                left,
                right,
                dst,
            } => {
                // For comparisons, we need to use cmpl + setcc
                // Handle memory-to-memory comparisons by loading left into register first
                let (left_reg, right_reg) = match (left, right) {
                    (asm::Operand::Stack(_), asm::Operand::Stack(_)) => {
                        // Both are memory locations, load left into register
                        write!(self.writer, "movl ")?;
                        self.operand(left)?;
                        write!(self.writer, ", %eax")?;
                        writeln!(self.writer)?;
                        (asm::Operand::Register(asm::Register::AX), right)
                    }
                    (left, right) => (left, right),
                };

                write!(self.writer, "cmpl ")?;
                self.operand(right_reg)?;
                write!(self.writer, ", ")?;
                self.operand(left_reg)?;
                writeln!(self.writer)?;

                // Set the result based on the comparison
                write!(self.writer, "set")?;
                match op {
                    asm::ComparisonOperator::Equal => write!(self.writer, "e")?,
                    asm::ComparisonOperator::NotEqual => write!(self.writer, "ne")?,
                    asm::ComparisonOperator::LessThan => write!(self.writer, "l")?,
                    asm::ComparisonOperator::LessThanOrEqual => write!(self.writer, "le")?,
                    asm::ComparisonOperator::GreaterThan => write!(self.writer, "g")?,
                    asm::ComparisonOperator::GreaterThanOrEqual => write!(self.writer, "ge")?,
                }
                write!(self.writer, " %al")?;
                writeln!(self.writer)?;

                // Move the result from AL to the destination (as 32-bit)
                write!(self.writer, "movzbl %al, %eax")?;
                writeln!(self.writer)?;
                write!(self.writer, "movl %eax, ")?;
                self.operand(dst)?;
                writeln!(self.writer)?;
            }
            asm::Instruction::Idiv { src } => {
                write!(self.writer, "idivl ")?;
                self.operand(src)?;
                writeln!(self.writer)?;
            }
            asm::Instruction::Cdq => {
                writeln!(self.writer, "cdq")?;
            }
            asm::Instruction::Label(text) => {
                writeln!(self.writer, "{text}:")?;
            }
            asm::Instruction::Jump { target } => {
                writeln!(self.writer, "jmp {target}")?;
            }
            asm::Instruction::JumpIfZero { condition, target } => {
                match condition {
                    asm::Operand::Imm(imm) => {
                        // For immediate values, we need to load into a register first
                        write!(self.writer, "movl ${imm}, %eax")?;
                        writeln!(self.writer)?;
                        write!(self.writer, "testl %eax, %eax")?;
                        writeln!(self.writer)?;
                    }
                    asm::Operand::Stack(_) => {
                        // Load stack value into register first to avoid memory-to-memory operations
                        write!(self.writer, "movl ")?;
                        self.operand(condition)?;
                        write!(self.writer, ", %eax")?;
                        writeln!(self.writer)?;
                        write!(self.writer, "testl %eax, %eax")?;
                        writeln!(self.writer)?;
                    }
                    _ => {
                        write!(self.writer, "testl ")?;
                        self.operand(condition)?;
                        write!(self.writer, ", ")?;
                        self.operand(condition)?;
                        writeln!(self.writer)?;
                    }
                }
                write!(self.writer, "jz {target}")?;
                writeln!(self.writer)?;
            }
            asm::Instruction::JumpIfNotZero { condition, target } => {
                match condition {
                    asm::Operand::Imm(imm) => {
                        // For immediate values, we need to load into a register first
                        write!(self.writer, "movl ${imm}, %eax")?;
                        writeln!(self.writer)?;
                        write!(self.writer, "testl %eax, %eax")?;
                        writeln!(self.writer)?;
                    }
                    asm::Operand::Stack(_) => {
                        // Load stack value into register first to avoid memory-to-memory operations
                        write!(self.writer, "movl ")?;
                        self.operand(condition)?;
                        write!(self.writer, ", %eax")?;
                        writeln!(self.writer)?;
                        write!(self.writer, "testl %eax, %eax")?;
                        writeln!(self.writer)?;
                    }
                    _ => {
                        write!(self.writer, "testl ")?;
                        self.operand(condition)?;
                        write!(self.writer, ", ")?;
                        self.operand(condition)?;
                        writeln!(self.writer)?;
                    }
                }
                write!(self.writer, "jnz {target}")?;
                writeln!(self.writer)?;
            }
        }

        Ok(())
    }

    fn operand(&mut self, operand: asm::Operand) -> fmt::Result {
        match operand {
            asm::Operand::Imm(imm) => write!(self.writer, "${imm}"),
            asm::Operand::Register(reg) => self.register(reg),
            asm::Operand::Stack(stack) => write!(self.writer, "-{}(%rbp)", stack + 4),
        }
    }

    fn register(&mut self, reg: asm::Register) -> fmt::Result {
        match reg {
            asm::Register::AX => write!(self.writer, "%eax"),
            asm::Register::DX => write!(self.writer, "%edx"),
            asm::Register::R10 => write!(self.writer, "%r10d"),
        }
    }

    fn unary_operator(&mut self, op: asm::UnaryOperator) -> fmt::Result {
        match op {
            asm::UnaryOperator::Neg => write!(self.writer, "negl"),
            asm::UnaryOperator::Complement => write!(self.writer, "notl"),
            asm::UnaryOperator::Not => {
                // Logical NOT: compare with 0 and set result to 1 if zero, 0 if non-zero
                write!(self.writer, "cmpl $0, ")?;
                Ok(())
            }
        }
    }

    fn binary_operator(&mut self, op: asm::BinaryOperator) -> fmt::Result {
        match op {
            asm::BinaryOperator::Add => write!(self.writer, "addl"),
            asm::BinaryOperator::Sub => write!(self.writer, "subl"),
            asm::BinaryOperator::Mul => write!(self.writer, "imull"),
            asm::BinaryOperator::And => write!(self.writer, "andl"),
            asm::BinaryOperator::Or => write!(self.writer, "orl"),
            asm::BinaryOperator::LeftShift => write!(self.writer, "shll"),
            asm::BinaryOperator::RightShift => write!(self.writer, "shrl"),
        }
    }
}
