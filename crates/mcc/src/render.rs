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
                self.unary_operator(op)?;
                write!(self.writer, " ")?;
                self.operand(operand)?;
                writeln!(self.writer)?;
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
            asm::Instruction::Idiv { src } => {
                write!(self.writer, "idivl ")?;
                self.operand(src)?;
                writeln!(self.writer)?;
            }
            asm::Instruction::Cdq => {
                writeln!(self.writer, "cdq")?;
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
            asm::UnaryOperator::Not => write!(self.writer, "notl"),
        }
    }

    fn binary_operator(&mut self, op: asm::BinaryOperator) -> fmt::Result {
        match op {
            asm::BinaryOperator::Add => write!(self.writer, "addl"),
            asm::BinaryOperator::Sub => write!(self.writer, "subl"),
            asm::BinaryOperator::Mul => write!(self.writer, "imull"),
        }
    }
}
