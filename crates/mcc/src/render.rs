use std::{
    borrow::Cow,
    fmt::{self, Write},
};

use target_lexicon::{OperatingSystem, Triple};

use crate::{Db, Text, codegen::asm};

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
pub struct AssemblyRenderer<W> {
    target: Triple,
    writer: W,
}

impl<W: Write> AssemblyRenderer<W> {
    pub fn new(target: Triple, writer: W) -> Self {
        Self { target, writer }
    }

    pub fn program(&mut self, db: &dyn Db, program: asm::Program) -> fmt::Result {
        for function in program.functions(db) {
            self.render_function(db, function)?;
            writeln!(self.writer)?;
        }

        if self.target.operating_system == OperatingSystem::Linux {
            writeln!(self.writer, ".section .note.GNU-stack, \"\", @progbits")?;
        }

        Ok(())
    }

    pub fn function_name<'a>(&self, name: &'a str) -> Cow<'a, str> {
        if matches!(self.target.operating_system, OperatingSystem::MacOSX(_)) {
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

        for instruction in function.instructions(db) {
            write!(self.writer, "  ")?;
            self.render_instruction(instruction)?;
            writeln!(self.writer)?;
        }

        Ok(())
    }

    pub fn render_instruction(&mut self, instruction: asm::Instruction) -> fmt::Result {
        match instruction {
            asm::Instruction::AllocateStack(_size) => {
                todo!();
            }
            asm::Instruction::Mov { src, dst } => {
                write!(self.writer, "mov ")?;
                self.operand(src)?;
                write!(self.writer, ", ")?;
                self.operand(dst)?;
            }
            asm::Instruction::Unary { op, operand } => {
                self.unary_operator(op)?;
                write!(self.writer, " ")?;
                self.operand(operand)?;
            }
            asm::Instruction::Ret => {
                write!(self.writer, "ret")?;
            }
        }

        Ok(())
    }

    pub fn operand(&mut self, operand: asm::Operand) -> fmt::Result {
        match operand {
            asm::Operand::Imm(imm) => write!(self.writer, "${imm}"),
            asm::Operand::Register(reg) => self.register(reg),
            asm::Operand::Pseudo(pseudo) => write!(self.writer, "${pseudo}"),
            asm::Operand::Stack(stack) => write!(self.writer, "${stack}"),
        }
    }

    pub fn register(&mut self, reg: asm::Register) -> fmt::Result {
        match reg {
            asm::Register::AX => write!(self.writer, "%eax"),
            asm::Register::R10 => write!(self.writer, "%r10"),
        }
    }

    pub fn unary_operator(&mut self, op: asm::UnaryOperator) -> fmt::Result {
        match op {
            asm::UnaryOperator::Neg => write!(self.writer, "neg"),
            asm::UnaryOperator::Not => write!(self.writer, "not"),
        }
    }
}
