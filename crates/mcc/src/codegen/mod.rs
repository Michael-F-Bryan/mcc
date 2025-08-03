//! Compile [Three Address Code](crate::lowering::tacky) to [Assembly](asm).

pub mod asm;

use std::{
    borrow::Cow,
    fmt::{self, Write},
};

use target_lexicon::{OperatingSystem, Triple};

use crate::{Db, Text, lowering::tacky};

/// Compile a parsed C program into assembly.
#[tracing::instrument(level = "info", skip_all)]
#[salsa::tracked]
pub fn generate_assembly(db: &dyn Db, program: tacky::Program<'_>, target: Triple) -> Text {
    let mut functions = Vec::new();
    for function in program.functions(db) {
        functions.push(lower_function(db, function));
    }

    let lowered = asm::Program::new(db, functions);
    render_program(db, lowered, target).unwrap()
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

#[tracing::instrument(level = "debug", skip_all, fields(target = %target))]
#[salsa::tracked]
fn render_program<'db>(
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
