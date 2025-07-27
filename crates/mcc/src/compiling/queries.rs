use mcc_syntax::nodes as ast;
use type_sitter::{HasChildren, HasOptionalChild, Node};

use super::types::Program;
use crate::{
    Db, Text,
    compiling::{FunctionDefinition, Instruction, Mov, Operand, Ret},
    types::{Ast, SourceFile},
};

/// Compile a parsed C program into assembly.
#[salsa::tracked]
pub fn compile(db: &dyn Db, ast: Ast<'_>, file: SourceFile) -> Text {
    let lowered = lower(db, ast, file);
    lowered.render(db)
}

#[salsa::tracked]
fn lower<'db>(db: &'db dyn Db, ast: Ast<'db>, file: SourceFile) -> Program<'db> {
    let translation_unit = ast.root(db);
    let src = file.contents(db);

    let mut cursor = translation_unit.walk();
    let mut functions = Vec::new();

    for child in translation_unit
        .children(&mut cursor)
        .filter_map(|c| c.ok())
    {
        type Child<'db> = <ast::TranslationUnit<'db> as HasChildren<'db>>::Child;
        match child {
            Child::FunctionDefinition(f) => {
                if let Some(f) = lower_function(db, f, src) {
                    functions.push(f);
                }
            }
            _ => todo!(),
        }
    }

    assert_eq!(functions.len(), 1);
    let main = functions.pop().unwrap();
    assert_eq!(main.name(db).as_str(), "main");

    Program::new(db, main)
}

fn lower_function<'db>(
    db: &'db dyn Db,
    f: ast::FunctionDefinition<'db>,
    src: &'db str,
) -> Option<FunctionDefinition<'db>> {
    let signature = f.declarator().ok()?.as_function_declarator()?;
    let name = signature
        .declarator()
        .ok()?
        .as_identifier()?
        .utf8_text(src.as_bytes())
        .ok()?;

    let mut instructions = Vec::new();

    let compound_statement = f.body().ok()?.into_raw();
    let mut cursor = compound_statement.walk();
    let children = compound_statement
        .children(&mut cursor)
        .filter_map(|c| ast::Statement::try_from_raw(c).ok());

    for child in children {
        eprintln!("{}", child.to_sexp());

        match child {
            ast::Statement::ReturnStatement(r) => {
                type Expr<'db> = <ast::ReturnStatement<'db> as HasOptionalChild<'db>>::Child;

                match r.child().and_then(|c| c.ok()) {
                    Some(Expr::Expression(ast::Expression::NumberLiteral(literal))) => {
                        let ret_value = literal.utf8_text(src.as_bytes()).ok()?.parse().unwrap();
                        instructions.push(Instruction::Mov(Mov {
                            src: Operand::Imm(ret_value),
                            dst: Operand::Register,
                        }));
                    }
                    Some(Expr::Expression(_)) => todo!(),
                    Some(Expr::CommaExpression(_)) => todo!(),
                    None => todo!(),
                }

                instructions.push(Instruction::Ret(Ret));
            }
            _ => todo!(),
        }
    }

    eprintln!("{name} => {instructions:?}");

    Some(FunctionDefinition::new(db, name.into(), instructions))
}
