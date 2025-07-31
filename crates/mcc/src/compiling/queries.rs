use codespan_reporting::diagnostic::Label;
use mcc_syntax::ast;
use target_lexicon::Triple;
use type_sitter::{HasChildren, Node, TreeCursor};

use crate::{
    Db, Text,
    compiling::{FunctionDefinition, Instruction, Mov, Operand, Program, Ret},
    diagnostics::{Diagnostic, DiagnosticExt, codes},
    types::{Ast, SourceFile},
};

/// Compile a parsed C program into assembly.
#[salsa::tracked]
pub fn compile(db: &dyn Db, ast: Ast<'_>, file: SourceFile, target: Triple) -> Text {
    let lowered = lower(db, ast, file);
    render_asm(db, lowered, target)
}

#[salsa::tracked]
pub fn render_asm(db: &dyn Db, program: Program<'_>, target: Triple) -> Text {
    program.render(db, target)
}

#[salsa::tracked]
fn lower<'db>(db: &'db dyn Db, ast: Ast<'db>, file: SourceFile) -> Program<'db> {
    let translation_unit = ast.root(db);

    let mut cursor = translation_unit.walk();
    let mut functions = Vec::new();

    for child in translation_unit
        .children(&mut cursor)
        .filter_map(|c| c.ok())
    {
        type Child<'db> = <ast::TranslationUnit<'db> as HasChildren<'db>>::Child;
        match child {
            Child::FunctionDefinition(f) => {
                if let Some(f) = lower_function(db, f, file) {
                    functions.push(f);
                }
            }
            _ => todo!(),
        }
    }

    match functions.as_slice() {
        [] => {
            let diagnostic = Diagnostic::error()
                .with_message("The program must contain a valid `main` function")
                .with_labels(vec![
                    Label::primary(file, translation_unit.span())
                        .with_message("error occurred here"),
                ]);
            diagnostic.accumulate(db);
        }
        [main] if main.name(db) == "main" => {
            // Happy path
        }
        [..] => {
            for func in &functions {
                if func.name(db).as_str() == "main" {
                    continue;
                }

                let diagnostic = Diagnostic::error()
                    .with_message("Only a `main` function is supported")
                    .with_labels(vec![
                        Label::primary(file, func.span(db)).with_message("error occurred here"),
                    ]);
                diagnostic.accumulate(db);
            }
        }
    }

    Program::new(db, functions)
}

fn lower_function<'db>(
    db: &'db dyn Db,
    f: ast::FunctionDefinition<'db>,
    file: SourceFile,
) -> Option<FunctionDefinition<'db>> {
    let signature: ast::FunctionDeclarator<'db> = f.declarator().ok()?.as_function_declarator()?;
    let ident: ast::Identifier<'db> = signature.declarator().ok()?.as_identifier()?;
    let src = file.contents(db);
    let name = ident.utf8_text(src.as_bytes()).ok()?;

    let mut instructions = Vec::new();

    let body: ast::CompoundStatement<'db> = f.body().ok()?;
    let mut cursor: TreeCursor<'db> = body.walk();

    for child in body
        .raw()
        .children(&mut cursor.0)
        .filter_map(|c| ast::Statement::try_from_raw(c).ok())
    {
        let mut cursor: TreeCursor<'db> = child.walk();
        match child {
            ast::Statement::ReturnStatement(r) => {
                match r
                    .raw()
                    .children(&mut cursor.0)
                    .find_map(|c| ast::Expression::try_from_raw(c).ok())
                {
                    Some(ast::Expression::NumberLiteral(literal)) => {
                        let ret_value = literal.utf8_text(src.as_bytes()).ok()?.parse().unwrap();
                        instructions.push(Instruction::Mov(Mov {
                            src: Operand::Imm(ret_value),
                            dst: Operand::Register,
                        }));
                    }
                    Some(other) => {
                        let diagnostic = Diagnostic::bug()
                            .with_message("Expected a number literal, but found something else")
                            .with_code(codes::types::UNIMPLEMENTED)
                            .with_labels(vec![
                                Label::primary(file, other.span())
                                    .with_message("error occurred here"),
                            ]);
                        diagnostic.accumulate(db);
                        return None;
                    }
                    None => todo!(),
                }

                instructions.push(Instruction::Ret(Ret));
            }
            other => todo!("{:?}", other),
        }
    }

    assert!(!instructions.is_empty());

    Some(FunctionDefinition::new(
        db,
        name.into(),
        instructions,
        f.span(),
    ))
}
