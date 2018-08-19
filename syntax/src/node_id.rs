use crate::ast::*;
use crate::visitor::{self, MutVisitor};
use heapsize_derive::HeapSizeOf;

/// A unique ID which corresponds to a particular AST node.
///
/// As a special case, `NodeId(0)` is an invalid node ID. This allows it to
/// be used as a placeholder.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, HeapSizeOf)]
pub struct NodeId(usize);

impl NodeId {
    fn new(n: usize) -> NodeId {
        NodeId(n)
    }

    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }

    pub fn placeholder() -> NodeId {
        NodeId::new(0)
    }
}

pub fn assign_node_ids(file: &mut File) {
    let mut gen = NodeIdGenerator::new();
    gen.visit_file_mut(file);
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct NodeIdGenerator {
    last_id: usize,
}

impl NodeIdGenerator {
    pub fn new() -> Self {
        NodeIdGenerator::default()
    }

    pub fn next_id(&mut self) -> NodeId {
        self.last_id += 1;
        NodeId::new(self.last_id)
    }
}

impl MutVisitor for NodeIdGenerator {
    fn visit_file_mut(&mut self, file: &mut File) {
        file.node_id = self.next_id();
        visitor::visit_file_mut(self, file);
    }

    fn visit_item_mut(&mut self, item: &mut Item) {
        visitor::visit_item_mut(self, item);
    }

    fn visit_function_mut(&mut self, func: &mut Function) {
        func.node_id = self.next_id();
        visitor::visit_function_mut(self, func);
    }

    fn visit_fn_decl_mut(&mut self, decl: &mut FnDecl) {
        decl.node_id = self.next_id();
        visitor::visit_fn_decl_mut(self, decl);
    }

    fn visit_ident_mut(&mut self, ident: &mut Ident) {
        ident.node_id = self.next_id();
    }

    fn visit_statement_mut(&mut self, stmt: &mut Statement) {
        visitor::visit_statement_mut(self, stmt);
    }

    fn visit_argument_mut(&mut self, arg: &mut Argument) {
        arg.node_id = self.next_id();
        visitor::visit_argument_mut(self, arg);
    }

    fn visit_return_mut(&mut self, ret: &mut Return) {
        ret.node_id = self.next_id();
        visitor::visit_return_mut(self, ret);
    }

    fn visit_expression_mut(&mut self, expr: &mut Expression) {
        visitor::visit_expression_mut(self, expr);
    }

    fn visit_literal_mut(&mut self, lit: &mut Literal) {
        lit.node_id = self.next_id();
    }

    fn visit_type_mut(&mut self, ty: &mut Type) {
        visitor::visit_type_mut(self, ty);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::AstNode;
    use crate::grammar::FileParser;
    use crate::visitor::Visitor;
    use regex::Regex;

    fn int_main_void() -> File {
        let src = "int main() { return 42; }";
        FileParser::new().parse(src).unwrap()
    }

    #[test]
    fn all_nodes_get_an_id() {
        let mut ast = int_main_void();

        assign_node_ids(&mut ast);

        let mut cb = |node: &dyn AstNode| assert!(node.node_id().is_valid(), "{:?}", node);
        cb.visit_file(&ast);

        // double check by printing and manually searching the string for the
        // placeholder
        let repr = format!("{:#?}", ast);
        let pat = Regex::new(r"NodeId\s*\(\s*0\s*\)").unwrap();

        if let Some(matched) = pat.find(&repr) {
            let before = &repr[..matched.start()];
            let line_no = before.matches('\n').count();
            panic!(
                "Invalid Node ID found around line {} in:\n{}",
                line_no, repr
            );
        }
    }
}
