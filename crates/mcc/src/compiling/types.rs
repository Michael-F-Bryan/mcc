use std::fmt::{self, Display};

use crate::{Db, Text};

#[salsa::tracked]
pub struct Program<'db> {
    pub main: FunctionDefinition<'db>,
}

#[salsa::tracked]
impl<'db> Program<'db> {
    pub fn render(self, db: &'db dyn Db) -> Text {
        self.display(db).to_string().into()
    }
}

impl<'db> Program<'db> {
    fn display(self, db: &'db dyn Db) -> impl Display {
        struct Repr<'db>(&'db dyn Db, Program<'db>);

        impl<'db> Display for Repr<'db> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let Repr(db, program) = *self;
                let main = program.main(db).render(db);

                writeln!(f, ".globl main")?;
                f.write_str(&main)?;

                Ok(())
            }
        }

        Repr(db, self)
    }
}

#[salsa::tracked]
pub struct FunctionDefinition<'db> {
    pub name: Text,
    pub instructions: Vec<Instruction>,
}

#[salsa::tracked]
impl<'db> FunctionDefinition<'db> {
    pub fn render(self, db: &'db dyn Db) -> Text {
        self.display(db).to_string().into()
    }
}

impl<'db> FunctionDefinition<'db> {
    fn display(self, db: &'db dyn Db) -> impl Display {
        struct Repr<'db>(&'db dyn Db, FunctionDefinition<'db>);

        impl<'db> Display for Repr<'db> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let Self(db, fd) = *self;
                let name = fd.name(db);
                let instructions = fd.instructions(db);

                writeln!(f, "{}:", name)?;

                for instruction in instructions {
                    writeln!(f, "  {}", instruction)?;
                }

                Ok(())
            }
        }

        Repr(db, self)
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
