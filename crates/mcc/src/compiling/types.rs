use std::fmt::{self, Display};

use mcc_syntax::Span;
use target_lexicon::{OperatingSystem, Triple};

use crate::{Db, Text};

#[salsa::tracked]
pub struct Program<'db> {
    pub functions: Vec<FunctionDefinition<'db>>,
}

#[salsa::tracked]
impl<'db> Program<'db> {
    pub fn render(self, db: &'db dyn Db, target: Triple) -> Text {
        self.display(db, &target).to_string().into()
    }
}

impl<'db> Program<'db> {
    fn display<'a>(self, db: &'db dyn Db, target: &'a Triple) -> impl Display + 'a
    where
        'db: 'a,
    {
        struct Repr<'b>(&'b dyn Db, Program<'b>, &'b Triple);

        impl<'b> Display for Repr<'b> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let Repr(db, program, target) = *self;

                writeln!(f, ".globl main")?;

                for function in program.functions(db) {
                    function.display(db, target).fmt(f)?;
                }

                if target.operating_system == OperatingSystem::Linux {
                    writeln!(f, ".section .note.GNU-stack, \"\", @progbits")?;
                }

                Ok(())
            }
        }

        Repr(db, self, target)
    }
}

#[salsa::tracked]
pub struct FunctionDefinition<'db> {
    pub name: Text,
    pub instructions: Vec<Instruction>,
    pub span: Span,
}

#[salsa::tracked]
impl<'db> FunctionDefinition<'db> {
    pub fn render(self, db: &'db dyn Db, target: &Triple) -> Text {
        self.display(db, target).to_string().into()
    }
}

impl<'db> FunctionDefinition<'db> {
    fn display<'a>(self, db: &'db dyn Db, target: &'a Triple) -> impl Display + 'a
    where
        'db: 'a,
    {
        struct Repr<'a>(&'a dyn Db, FunctionDefinition<'a>, &'a Triple);

        impl<'a> Display for Repr<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let Self(db, fd, target) = *self;
                let name = fd.name(db);
                let instructions = fd.instructions(db);

                // Note: macOS requires functions to be prefixed with an
                // underscore.
                if matches!(target.operating_system, OperatingSystem::Darwin(_)) {
                    write!(f, "_")?;
                }

                writeln!(f, "{name}:")?;

                for instruction in instructions {
                    writeln!(f, "  {instruction}")?;
                }

                Ok(())
            }
        }

        Repr(db, self, target)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Instruction {
    Mov(Mov),
    Ret(Ret),
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Mov(mov) => Display::fmt(mov, f),
            Instruction::Ret(ret) => Display::fmt(ret, f),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Mov {
    pub src: Operand,
    pub dst: Operand,
}

impl Display for Mov {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Mov { src, dst } = self;
        write!(f, "mov {src}, {dst}")
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Ret;

impl Display for Ret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ret")
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Operand {
    Imm(i32),
    Register,
}

impl Display for Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operand::Imm(imm) => write!(f, "${imm}"),
            Operand::Register => write!(f, "%eax"),
        }
    }
}
