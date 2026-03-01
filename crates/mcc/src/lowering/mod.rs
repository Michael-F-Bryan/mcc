//! Lower from an [Abstract Syntax Tree](mcc_syntax::ast) to [Three Address Code](tacky).

use codespan_reporting::diagnostic::Label;
use mcc_syntax::{Span, ast};
use type_sitter::{HasChild, HasChildren, HasOptionalChild, Node, TreeCursor};

use crate::{
    Db, codes,
    diagnostics::{Diagnostic, DiagnosticExt},
    typechecking::hir,
    types::{Ast, SourceFile},
};

pub mod tacky;

impl<'db> hir::TranslationUnit<'db> {
    pub fn lower(self, db: &'db dyn Db) -> tacky::Program<'db> {
        let functions = self
            .items(db)
            .iter()
            .filter_map(|item| lower_item(db, *item, self.file(db)))
            .collect();
        tacky::Program::new(db, functions)
    }
}

/// Lower a single HIR item to TACKY. Cached per (item, file).
#[salsa::tracked]
pub fn lower_item<'db>(
    db: &'db dyn Db,
    item: hir::Item<'db>,
    file: SourceFile,
) -> Option<tacky::FunctionDefinition<'db>> {
    match item {
        hir::Item::Function(f) => lower_hir_function(db, file, f),
    }
}

/// Lower an [Abstract Syntax Tree](mcc_syntax::ast) to our [Three Address Code](tacky)
/// intermediate representation.
///
/// Prefer [`lower_program`] which runs typecheck then HIR-based lowering; this function
/// bypasses the typecheck stage and is retained for tests or legacy use.
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
                    .with_message("Translation unit item not implemented")
                    .with_code(codes::type_check::unimplemented)
                    .with_labels(vec![
                        Label::primary(file, Span::for_node(*other.raw()))
                            .with_message(other.kind()),
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
            other => {
                let diagnostic = Diagnostic::bug()
                    .with_message("Statement not implemented")
                    .with_code(codes::type_check::unimplemented)
                    .with_labels(vec![
                        Label::primary(self.file, other.span()).with_message(other.kind()),
                    ]);
                diagnostic.accumulate(self.db);
            }
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

    /// Lower an expression, returning a [`tacky::Val`] containing the result if successful.
    fn lower_expression(&mut self, expr: ast::Expression<'_>) -> Option<tacky::Val> {
        match expr {
            ast::Expression::NumberLiteral(literal) => self.lower_number_literal(literal),
            ast::Expression::UnaryExpression(unary) => self.lower_unary_expression(unary),
            ast::Expression::BinaryExpression(binary) => self.lower_binary_expression(binary),
            ast::Expression::ParenthesizedExpression(expr) => {
                match expr.child().ok()? {
                    ast::anon_unions::CommaExpression_CompoundStatement_Expression_PreprocDefined::Expression(expr) => {
                        self.lower_expression(expr)
                    },
                    _ => {
                        let diagnostic = Diagnostic::bug()
                            .with_message("Unexpected item in parenthesized expression")
                            .with_code(codes::type_check::unimplemented)
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
                    .with_message("Expression not implemented")
                    .with_code(codes::type_check::unimplemented)
                    .with_labels(vec![
                        Label::primary(self.file, other.span()).with_message(other.kind()),
                    ]);
                diagnostic.accumulate(self.db);
                None
            }
        }
    }

    fn lower_binary_expression(&mut self, binary: ast::BinaryExpression<'_>) -> Option<tacky::Val> {
        let left = binary.left().ok()?.as_expression()?;
        let right = binary.right().ok()?.as_expression()?;

        type Op<'a> = ast::anon_unions::NotEq_Mod_And_AndAnd_Mul_Add_Sub_Div_Lt_LtLt_LtEq_EqEq_Gt_GtEq_GtGt_BitXor_Or_OrOr<'a>;

        match binary.operator().ok()? {
            // Logical AND with short-circuit evaluation
            Op::AndAnd(_) => self.lower_logical_and(left, right),
            // Logical OR with short-circuit evaluation
            Op::OrOr(_) => self.lower_logical_or(left, right),
            // Regular binary operators
            op => {
                let left = self.lower_expression(left)?;
                let right = self.lower_expression(right)?;

                match op {
                    Op::Add(_) => {
                        self.lower_binary_operator(left, right, tacky::BinaryOperator::Add)
                    }
                    Op::Sub(_) => {
                        self.lower_binary_operator(left, right, tacky::BinaryOperator::Sub)
                    }
                    Op::Mul(_) => {
                        self.lower_binary_operator(left, right, tacky::BinaryOperator::Mul)
                    }
                    Op::Div(_) => {
                        self.lower_binary_operator(left, right, tacky::BinaryOperator::Div)
                    }
                    Op::Mod(_) => {
                        self.lower_binary_operator(left, right, tacky::BinaryOperator::Mod)
                    }
                    Op::And(_) => {
                        self.lower_binary_operator(left, right, tacky::BinaryOperator::And)
                    }
                    Op::Or(_) => self.lower_binary_operator(left, right, tacky::BinaryOperator::Or),
                    Op::LtLt(_) => {
                        self.lower_binary_operator(left, right, tacky::BinaryOperator::LeftShift)
                    }
                    Op::GtGt(_) => {
                        self.lower_binary_operator(left, right, tacky::BinaryOperator::RightShift)
                    }
                    Op::EqEq(_) => {
                        self.lower_comparison(left, right, tacky::ComparisonOperator::Equal)
                    }
                    Op::NotEq(_) => {
                        self.lower_comparison(left, right, tacky::ComparisonOperator::NotEqual)
                    }
                    Op::Lt(_) => {
                        self.lower_comparison(left, right, tacky::ComparisonOperator::LessThan)
                    }
                    Op::LtEq(_) => self.lower_comparison(
                        left,
                        right,
                        tacky::ComparisonOperator::LessThanOrEqual,
                    ),
                    Op::Gt(_) => {
                        self.lower_comparison(left, right, tacky::ComparisonOperator::GreaterThan)
                    }
                    Op::GtEq(_) => self.lower_comparison(
                        left,
                        right,
                        tacky::ComparisonOperator::GreaterThanOrEqual,
                    ),
                    other => {
                        let diagnostic = Diagnostic::bug()
                            .with_message("Binary operator not implemented")
                            .with_code(codes::type_check::unimplemented)
                            .with_labels(vec![
                                Label::primary(self.file, binary.span()).with_message(other.kind()),
                            ]);
                        diagnostic.accumulate(self.db);
                        None
                    }
                }
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
            ast::anon_unions::Not_Add_Sub_BitNot::Not(_) => tacky::UnaryOperator::Not,
        };

        self.instructions.push(tacky::Instruction::Unary {
            op,
            src,
            dst: dst.clone(),
        });
        Some(dst)
    }

    /// Lower logical AND (&&) with short-circuit evaluation.
    ///
    /// For `left && right`:
    /// 1. Evaluate left
    /// 2. If left is zero, jump to false case and set result to 0
    /// 3. Otherwise, evaluate right and set result to 1 if right is truthy, 0 if falsy
    fn lower_logical_and(
        &mut self,
        left: ast::Expression<'_>,
        right: ast::Expression<'_>,
    ) -> Option<tacky::Val> {
        let left_val = self.lower_expression(left)?;

        // Create labels for control flow
        let false_label = self.label();
        let end_label = self.label();

        // Create result variable
        let result = tacky::Val::Var(self.temporary());

        // If left is zero, jump to false case
        self.instructions.push(tacky::Instruction::JumpIfZero {
            condition: left_val,
            target: false_label.clone(),
        });

        // Left is non-zero, evaluate right and convert to boolean (1 or 0)
        let right_val = self.lower_expression(right)?;
        let right_bool = tacky::Val::Var(self.temporary());

        // Convert right to boolean: cmpl $0, right; setne %al; movb %al, right_bool
        self.instructions.push(tacky::Instruction::Comparison {
            op: tacky::ComparisonOperator::NotEqual,
            left_src: tacky::Val::Constant(0),
            right_src: right_val,
            dst: right_bool.clone(),
        });

        self.instructions.push(tacky::Instruction::Copy {
            src: right_bool,
            dst: result.clone(),
        });

        // Jump to end
        self.instructions.push(tacky::Instruction::Jump {
            target: end_label.clone(),
        });

        // False case: set result to 0
        self.instructions
            .push(tacky::Instruction::Label(false_label));
        self.instructions.push(tacky::Instruction::Copy {
            src: tacky::Val::Constant(0),
            dst: result.clone(),
        });

        // End case
        self.instructions.push(tacky::Instruction::Label(end_label));

        Some(result)
    }

    /// Lower logical OR (||) with short-circuit evaluation.
    ///
    /// For `left || right`:
    /// 1. Evaluate left
    /// 2. If left is non-zero, jump to true case and set result to 1
    /// 3. Otherwise, evaluate right and set result to 1 if right is truthy, 0 if falsy
    fn lower_logical_or(
        &mut self,
        left: ast::Expression<'_>,
        right: ast::Expression<'_>,
    ) -> Option<tacky::Val> {
        let left_val = self.lower_expression(left)?;

        // Create labels for control flow
        let true_label = self.label();
        let end_label = self.label();

        // Create result variable
        let result = tacky::Val::Var(self.temporary());

        // If left is non-zero, jump to true case
        self.instructions.push(tacky::Instruction::JumpIfNotZero {
            condition: left_val.clone(),
            target: true_label.clone(),
        });

        // Left is zero, evaluate right and convert to boolean (1 or 0)
        let right_val = self.lower_expression(right)?;
        let right_bool = tacky::Val::Var(self.temporary());

        // Convert right to boolean: cmpl $0, right; setne %al; movb %al, right_bool
        self.instructions.push(tacky::Instruction::Comparison {
            op: tacky::ComparisonOperator::NotEqual,
            left_src: tacky::Val::Constant(0),
            right_src: right_val,
            dst: right_bool.clone(),
        });

        self.instructions.push(tacky::Instruction::Copy {
            src: right_bool,
            dst: result.clone(),
        });

        // Jump to end
        self.instructions.push(tacky::Instruction::Jump {
            target: end_label.clone(),
        });

        // True case: set result to 1
        self.instructions
            .push(tacky::Instruction::Label(true_label));
        self.instructions.push(tacky::Instruction::Copy {
            src: tacky::Val::Constant(1),
            dst: result.clone(),
        });

        // End case
        self.instructions.push(tacky::Instruction::Label(end_label));

        Some(result)
    }

    fn temporary(&mut self) -> tacky::Variable {
        let temp = tacky::Variable::Anonymous(self.next_anonymous);
        self.next_anonymous += 1;
        temp
    }

    fn label(&mut self) -> crate::Text {
        let label_name = format!("L{}", self.next_anonymous);
        self.next_anonymous += 1;
        label_name.into()
    }

    fn lower_binary_operator(
        &mut self,
        left: tacky::Val,
        right: tacky::Val,
        binary_op: tacky::BinaryOperator,
    ) -> Option<tacky::Val> {
        let dst = tacky::Val::Var(self.temporary());
        self.instructions.push(tacky::Instruction::Binary {
            op: binary_op,
            left_src: left,
            right_src: right,
            dst: dst.clone(),
        });

        Some(dst)
    }
    fn lower_comparison(
        &mut self,
        left: tacky::Val,
        right: tacky::Val,
        comparison_op: tacky::ComparisonOperator,
    ) -> Option<tacky::Val> {
        let dst = tacky::Val::Var(self.temporary());
        self.instructions.push(tacky::Instruction::Comparison {
            op: comparison_op,
            left_src: left,
            right_src: right,
            dst: dst.clone(),
        });

        Some(dst)
    }
}

/// Returns all diagnostics from the typecheck and lower stage (combined).
///
/// Call after [`lower_program`] to get a single list of diagnostics for the typecheck + HIR→TACKY
/// pipeline. The driver uses this so "after_lower" receives one consolidated list.
pub fn lower_stage_diagnostics(
    db: &dyn Db,
    file: SourceFile,
) -> Vec<&crate::diagnostics::Diagnostics> {
    let typecheck_diags =
        crate::typechecking::typecheck::accumulated::<crate::diagnostics::Diagnostics>(db, file);
    let lower_diags = lower_program::accumulated::<crate::diagnostics::Diagnostics>(db, file);
    typecheck_diags.into_iter().chain(lower_diags).collect()
}

/// Lower a compilation unit from HIR to TACKY by first typechecking, then lowering each item.
///
/// This is the single entry point for the parse → typecheck → lower pipeline. Diagnostics from
/// both typecheck and lowering are available via [`lower_stage_diagnostics`].
#[tracing::instrument(level = "info", skip_all)]
#[salsa::tracked]
pub fn lower_program<'db>(db: &'db dyn Db, file: SourceFile) -> tacky::Program<'db> {
    let tu = crate::typechecking::typecheck(db, file);
    let file_ref = tu.file(db);
    let functions: Vec<_> = tu
        .items(db)
        .iter()
        .filter_map(|item| match item {
            hir::Item::Function(f) => lower_hir_function(db, file_ref, *f),
        })
        .collect();

    match functions.as_slice() {
        [] => {
            let ast = crate::parse(db, file);
            let tu_span = ast.root(db).span();
            Diagnostic::error()
                .with_message("The program must contain a valid `main` function")
                .with_labels(vec![
                    Label::primary(file_ref, tu_span).with_message("error occurred here"),
                ])
                .accumulate(db);
        }
        [main] if main.name(db).as_str() == "main" => {}
        [..] => {
            for func in &functions {
                if func.name(db).as_str() == "main" {
                    continue;
                }
                Diagnostic::error()
                    .with_message("Only a `main` function is supported")
                    .with_labels(vec![
                        Label::primary(file_ref, func.span(db)).with_message("error occurred here"),
                    ])
                    .accumulate(db);
            }
        }
    }

    tacky::Program::new(db, functions)
}

/// Lower a single HIR function to TACKY. Cached per (file, function). Diagnostics are
/// accumulated on the database (attributed to the calling query, e.g. [`lower_program`]).
#[salsa::tracked]
fn lower_hir_function<'db>(
    db: &'db dyn Db,
    file: SourceFile,
    f: hir::FunctionDefinition<'db>,
) -> Option<tacky::FunctionDefinition<'db>> {
    let node = f.node(db).node(db);
    let body: ast::CompoundStatement<'db> = node.body().ok()?;
    let mut ctx = FunctionContext::new(db, file);
    ctx.lower_body(body);
    let name = f.name(db).text(db);
    Some(tacky::FunctionDefinition::new(
        db,
        name,
        ctx.instructions,
        node.span(),
    ))
}

#[cfg(test)]
mod tests {
    use crate::{Database, diagnostics::Diagnostics, types::SourceFile};

    use super::*;

    #[test]
    fn lower_program_produces_tacky_with_main() {
        let db = Database::default();
        let src = "int main(void) { return 0; }";
        let file = SourceFile::new(&db, "test.c".into(), src.into());

        let program = lower_program(&db, file);

        assert_eq!(program.functions(&db).len(), 1);
        assert_eq!(program.functions(&db)[0].name(&db).as_str(), "main");
    }

    #[test]
    fn lower_program_diagnoses_missing_main() {
        let db = Database::default();
        let src = "int foo(void) { return 0; }";
        let file = SourceFile::new(&db, "test.c".into(), src.into());

        let _ = lower_program(&db, file);
        let diags = lower_program::accumulated::<Diagnostics>(&db, file);

        assert!(
            diags
                .iter()
                .any(|d| format!("{d:?}").contains("Only a `main` function is supported")),
            "expected diagnostic about missing main, got: {:?}",
            diags
        );
    }

    #[test]
    fn lower_program_diagnoses_extra_functions() {
        let db = Database::default();
        let src = "int main(void) { return 0; } int foo(void) { return 1; }";
        let file = SourceFile::new(&db, "test.c".into(), src.into());

        let _ = lower_program(&db, file);
        let diags = lower_program::accumulated::<Diagnostics>(&db, file);

        assert!(
            diags
                .iter()
                .any(|d| format!("{d:?}").contains("Only a `main` function is supported")),
            "expected diagnostic about only main, got: {:?}",
            diags
        );
    }
}
