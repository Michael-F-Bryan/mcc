//! Lower from an [Abstract Syntax Tree](mcc_syntax::ast) to [Three Address Code](tacky).

use codespan_reporting::diagnostic::Label;
use mcc_syntax::{Span, ast};
use type_sitter::{HasChild, HasChildren, HasOptionalChild, Node, TreeCursor};

use crate::{
    Db,
    diagnostics::{Diagnostic, DiagnosticExt, codes},
    types::{Ast, SourceFile},
};

pub mod tacky;

/// Lower an [Abstract Syntax Tree](mcc_syntax::ast) to our [Three Address Code](tacky)
/// intermediate representation.
#[tracing::instrument(level = "info", skip_all)]
#[salsa::tracked]
pub fn lower<'db>(db: &'db dyn Db, ast: Ast<'db>, file: SourceFile) -> tacky::Program<'db> {
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
            other => {
                let diagnostic = Diagnostic::bug()
                    .with_message("Unexpected AST node")
                    .with_code(codes::type_check::UNIMPLEMENTED)
                    .with_labels(vec![
                        Label::primary(file, Span::for_node(*other.raw()))
                            .with_message(other.kind()),
                    ]);
                diagnostic.accumulate(db);
            }
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

    tacky::Program::new(db, functions)
}

#[tracing::instrument(skip_all)]
#[salsa::tracked]
fn lower_function<'db>(
    db: &'db dyn Db,
    f: ast::FunctionDefinition<'db>,
    file: SourceFile,
) -> Option<tacky::FunctionDefinition<'db>> {
    let signature: ast::FunctionDeclarator<'db> = f.declarator().ok()?.as_function_declarator()?;
    let ident: ast::Identifier<'db> = signature.declarator().ok()?.as_identifier()?;
    let src = file.contents(db);
    let name = ident.utf8_text(src.as_bytes()).ok()?;

    let body: ast::CompoundStatement<'db> = f.body().ok()?;

    let mut ctx = FunctionContext::new(db, file);
    ctx.lower_body(body);

    Some(tacky::FunctionDefinition::new(
        db,
        name.into(),
        ctx.instructions,
        f.span(),
    ))
}

struct FunctionContext<'db> {
    db: &'db dyn Db,
    file: SourceFile,
    instructions: Vec<tacky::Instruction>,
    next_anonymous: u32,
}

impl<'db> FunctionContext<'db> {
    fn new(db: &'db dyn Db, file: SourceFile) -> Self {
        Self {
            db,
            file,
            instructions: Vec::new(),
            next_anonymous: 0,
        }
    }
    fn lower_body(&mut self, body: ast::CompoundStatement<'db>) {
        let mut cursor: TreeCursor<'db> = body.walk();

        for child in body
            .children(&mut cursor)
            .filter_map(|c| c.ok())
            .filter_map(|c| c.as_statement())
        {
            self.lower_statement(child);
        }
    }

    fn lower_statement(&mut self, statement: ast::Statement<'db>) {
        match statement {
            ast::Statement::ReturnStatement(r) => {
                self.lower_return_statement(r);
            }
            other => todo!("{:?}", other),
        }
    }

    fn lower_return_statement(&mut self, r: ast::ReturnStatement<'db>) -> Option<()> {
        match r
            .child()
            .and_then(|c| c.ok())
            .and_then(|c| c.as_expression())
        {
            Some(expr) => {
                let ret = self.lower_expression(expr)?;
                self.instructions.push(tacky::Instruction::Return(ret));
            }
            None => todo!(),
        }

        Some(())
    }

    fn lower_expression(&mut self, expr: ast::Expression<'_>) -> Option<tacky::Val> {
        match expr {
            ast::Expression::NumberLiteral(literal) => self.lower_number_literal(literal),
            ast::Expression::UnaryExpression(unary) => self.lower_unary_expression(unary),
            ast::Expression::ParenthesizedExpression(expr) => {
                match expr.child().ok()? {
                    ast::anon_unions::CommaExpression_CompoundStatement_Expression_PreprocDefined::Expression(expr) => {
                        self.lower_expression(expr)
                    },
                    _ => {
                        let diagnostic = Diagnostic::bug()
                            .with_message("Unexpected AST node")
                            .with_code(codes::type_check::UNIMPLEMENTED)
                            .with_labels(vec![
                                Label::primary(self.file, expr.span()).with_message(expr.kind()),
                            ]);
                        diagnostic.accumulate(self.db);
                        None
                    },
                }
            }
            other => {
                let diagnostic = Diagnostic::bug()
                    .with_message("Unexpected AST node")
                    .with_code(codes::type_check::UNIMPLEMENTED)
                    .with_labels(vec![
                        Label::primary(self.file, other.span()).with_message(other.kind()),
                    ]);
                diagnostic.accumulate(self.db);
                None
            }
        }
    }

    fn lower_number_literal(&self, literal: ast::NumberLiteral<'_>) -> Option<tacky::Val> {
        let src = self.file.contents(self.db);
        let value = literal.utf8_text(src.as_bytes()).ok()?.parse().unwrap();
        Some(tacky::Val::Constant(value))
    }

    fn lower_unary_expression(&mut self, unary: ast::UnaryExpression<'_>) -> Option<tacky::Val> {
        let arg = unary.argument().ok()?.as_expression()?;
        let src = self.lower_expression(arg)?;

        let dst_name = self.temporary();
        let dst = tacky::Val::Var(dst_name);

        let op = match unary.operator().ok()? {
            ast::anon_unions::Not_Add_Sub_BitNot::Add(_) => {
                // No-op
                return Some(src);
            }
            ast::anon_unions::Not_Add_Sub_BitNot::Sub(_) => tacky::UnaryOperator::Negate,
            ast::anon_unions::Not_Add_Sub_BitNot::BitNot(_) => tacky::UnaryOperator::Complement,
            ast::anon_unions::Not_Add_Sub_BitNot::Not(_) => {
                todo!()
            }
        };

        self.instructions.push(tacky::Instruction::Unary {
            op,
            src,
            dst: dst.clone(),
        });
        Some(dst)
    }

    fn temporary(&mut self) -> tacky::Variable {
        let temp = tacky::Variable::Anonymous(self.next_anonymous);
        self.next_anonymous += 1;
        temp
    }
}
