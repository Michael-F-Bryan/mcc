//! Utilities for visiting each node on an AST.

use crate::ast::*;

pub trait MutVisitor {
    fn visit_file_mut(&mut self, file: &mut File) {
        visit_file_mut(self, file);
    }

    fn visit_item_mut(&mut self, item: &mut Item) {
        visit_item_mut(self, item);
    }

    fn visit_function_mut(&mut self, func: &mut Function) {
        visit_function_mut(self, func);
    }

    fn visit_fn_decl_mut(&mut self, decl: &mut FnDecl) {
        visit_fn_decl_mut(self, decl);
    }

    fn visit_ident_mut(&mut self, _ident: &mut Ident) {}

    fn visit_statement_mut(&mut self, stmt: &mut Statement) {
        visit_statement_mut(self, stmt);
    }

    fn visit_argument_mut(&mut self, arg: &mut Argument) {
        visit_argument_mut(self, arg);
    }

    fn visit_return_mut(&mut self, ret: &mut Return) {
        visit_return_mut(self, ret);
    }

    fn visit_expression_mut(&mut self, expr: &mut Expression) {
        visit_expression_mut(self, expr);
    }

    fn visit_literal_mut(&mut self, _lit: &mut Literal) {}

    fn visit_type_mut(&mut self, ty: &mut Type) {
        visit_type_mut(self, ty);
    }
}

pub fn visit_file_mut<V: MutVisitor + ?Sized>(visitor: &mut V, file: &mut File) {
    for item in &mut file.items {
        visitor.visit_item_mut(item);
    }
}

pub fn visit_item_mut<V: MutVisitor + ?Sized>(visitor: &mut V, item: &mut Item) {
    match item {
        Item::Function(func) => visitor.visit_function_mut(func),
        Item::u32(_) => unreachable!(),
    }
}

pub fn visit_function_mut<V: MutVisitor + ?Sized>(visitor: &mut V, func: &mut Function) {
    visitor.visit_fn_decl_mut(&mut func.signature);

    for stmt in &mut func.body {
        visitor.visit_statement_mut(stmt);
    }
}

pub fn visit_fn_decl_mut<V: MutVisitor + ?Sized>(visitor: &mut V, decl: &mut FnDecl) {
    visitor.visit_ident_mut(&mut decl.name);
    visitor.visit_type_mut(&mut decl.return_value);

    for arg in &mut decl.args {
        visitor.visit_argument_mut(arg);
    }
}

pub fn visit_statement_mut<V: MutVisitor + ?Sized>(visitor: &mut V, stmt: &mut Statement) {
    match stmt {
        Statement::Return(ret) => visitor.visit_return_mut(ret),
        Statement::u32(_) => unreachable!(),
    }
}

pub fn visit_return_mut<V: MutVisitor + ?Sized>(visitor: &mut V, ret: &mut Return) {
    if let Some(return_value) = ret.value.as_mut() {
        visitor.visit_expression_mut(return_value);
    }
}

pub fn visit_argument_mut<V: MutVisitor + ?Sized>(visitor: &mut V, arg: &mut Argument) {
    if let Some(name) = arg.name.as_mut() {
        visitor.visit_ident_mut(name);
    }

    visitor.visit_type_mut(&mut arg.ty);
}

pub fn visit_expression_mut<V: MutVisitor + ?Sized>(visitor: &mut V, expr: &mut Expression) {
    match expr {
        Expression::Literal(lit) => visitor.visit_literal_mut(lit),
        Expression::BinaryOp(bin_op) => {
            visitor.visit_expression_mut(&mut bin_op.left);
            visitor.visit_expression_mut(&mut bin_op.right);
        }
    }
}

pub fn visit_type_mut<V: MutVisitor + ?Sized>(visitor: &mut V, ty: &mut Type) {
    match ty {
        Type::Ident(id) => visitor.visit_ident_mut(id),
    }
}

pub trait Visitor {
    fn visit_any_ast_node(&mut self, _node: &dyn AstNode) {}

    fn visit_file(&mut self, file: &File) {
        visit_file(self, file);
    }

    fn visit_item(&mut self, item: &Item) {
        visit_item(self, item);
    }

    fn visit_function(&mut self, func: &Function) {
        visit_function(self, func);
    }

    fn visit_fn_decl(&mut self, decl: &FnDecl) {
        visit_fn_decl(self, decl);
    }

    fn visit_statement(&mut self, stmt: &Statement) {
        visit_statement(self, stmt);
    }

    fn visit_ident(&mut self, ident: &Ident) {
        visit_ident(self, ident);
    }

    fn visit_argument(&mut self, arg: &Argument) {
        visit_argument(self, arg);
    }

    fn visit_return(&mut self, ret: &Return) {
        visit_return(self, ret);
    }

    fn visit_expression(&mut self, expr: &Expression) {
        visit_expression(self, expr);
    }

    fn visit_type(&mut self, ty: &Type) {
        visit_type(self, ty);
    }

    fn visit_literal(&mut self, lit: &Literal) {
        visit_literal(self, lit);
    }
}

pub fn visit_file<V: Visitor + ?Sized>(visitor: &mut V, file: &File) {
    visitor.visit_any_ast_node(file);

    for item in &file.items {
        visitor.visit_item(item);
    }
}

pub fn visit_item<V: Visitor + ?Sized>(visitor: &mut V, item: &Item) {
    visitor.visit_any_ast_node(item);

    match item {
        Item::Function(func) => visitor.visit_function(func),
        Item::u32(_) => unreachable!(),
    }
}

pub fn visit_function<V: Visitor + ?Sized>(visitor: &mut V, func: &Function) {
    visitor.visit_any_ast_node(func);

    visitor.visit_fn_decl(&func.signature);

    for stmt in &func.body {
        visitor.visit_statement(stmt);
    }
}

pub fn visit_fn_decl<V: Visitor + ?Sized>(visitor: &mut V, decl: &FnDecl) {
    visitor.visit_any_ast_node(decl);
    visitor.visit_ident(&decl.name);
    visitor.visit_type(&decl.return_value);

    for arg in &decl.args {
        visitor.visit_argument(arg);
    }
}

pub fn visit_statement<V: Visitor + ?Sized>(visitor: &mut V, stmt: &Statement) {
    visitor.visit_any_ast_node(stmt);

    match stmt {
        Statement::Return(ret) => visitor.visit_return(ret),
        Statement::u32(_) => unreachable!(),
    }
}

pub fn visit_return<V: Visitor + ?Sized>(visitor: &mut V, ret: &Return) {
    visitor.visit_any_ast_node(ret);

    if let Some(return_value) = ret.value.as_ref() {
        visitor.visit_expression(return_value);
    }
}

pub fn visit_ident<V: Visitor + ?Sized>(visitor: &mut V, ident: &Ident) {
    visitor.visit_any_ast_node(ident);
}

pub fn visit_type<V: Visitor + ?Sized>(visitor: &mut V, ty: &Type) {
    visitor.visit_any_ast_node(ty);

    match ty {
        Type::Ident(id) => visitor.visit_ident(id),
    }
}

pub fn visit_literal<V: Visitor + ?Sized>(visitor: &mut V, lit: &Literal) {
    visitor.visit_any_ast_node(lit);
}

pub fn visit_argument<V: Visitor + ?Sized>(visitor: &mut V, arg: &Argument) {
    visitor.visit_any_ast_node(arg);

    if let Some(name) = arg.name.as_ref() {
        visitor.visit_ident(name);
    }
}

pub fn visit_expression<V: Visitor + ?Sized>(visitor: &mut V, expr: &Expression) {
    visitor.visit_any_ast_node(expr);

    match expr {
        Expression::Literal(lit) => visitor.visit_literal(lit),
        Expression::BinaryOp(bin_op) => {
            visitor.visit_expression(&bin_op.left);
            visitor.visit_expression(&bin_op.right);
        }
    }
}

impl<F> Visitor for F
where
    F: FnMut(&dyn AstNode),
{
    fn visit_any_ast_node(&mut self, node: &dyn AstNode) {
        self(node);
    }
}
