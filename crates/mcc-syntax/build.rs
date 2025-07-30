use proc_macro2::TokenStream;
use std::path::PathBuf;
use std::{env, fs};
use type_sitter_gen::{NodeTypeMap, generate_nodes};

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    println!("cargo::rerun-if-changed=build.rs");

    println!("cargo::rerun-if-changed=tree-sitter-c");

    let node_types = NodeTypeMap::try_from(tree_sitter_c::NODE_TYPES).unwrap();

    let type_paths = node_types
        .values()
        .map(|t| t.rust_type_path().into_owned())
        .collect::<Vec<_>>();

    let mut nodes_rs = generate_nodes(node_types).unwrap().into_string();

    // HACK: Anonymous unions generate types with massive names, which causes
    // rustdoc to fail.
    // See https://github.com/Jakobeha/type-sitter/issues/17 for more
    let long_type_names = &[
        "pub enum AttributedStatement_BreakStatement_CompoundStatement_ContinueStatement_Declaration_DoStatement_ExpressionStatement_ForStatement_GotoStatement_IfStatement_LabeledStatement_ReturnStatement_SehLeaveStatement_SehTryStatement_SwitchStatement_TypeDefinition_WhileStatement",
        "pub enum AttributedStatement_BreakStatement_CaseStatement_CompoundStatement_ContinueStatement_Declaration_DoStatement_ExpressionStatement_ForStatement_FunctionDefinition_GotoStatement_IfStatement_LabeledStatement_LinkageSpecification_PreprocCall_PreprocDef_PreprocFunctionDef_PreprocIf_PreprocIfdef_PreprocInclude_ReturnStatement_SwitchStatement_TypeDefinition_TypeSpecifier_WhileStatement",
    ];

    for search_term in long_type_names {
        let replacement = format!("#[doc(hidden)] {search_term}");
        nodes_rs = nodes_rs.replace(search_term, &replacement);
    }

    let mut extra_tokens = TokenStream::new();

    // Give each node type a `span` method.
    for type_path in type_paths {
        let ident: proc_macro2::TokenStream = type_path.parse().unwrap();

        let spanned_impl = quote::quote! {
            impl #ident<'_> {
                pub fn span(&self) -> crate::Span {
                    let node = type_sitter::Node::raw(self);
                    crate::Span::for_node(*node)
                }
            }
        };
        extra_tokens.extend(spanned_impl);
    }

    let extra_tokens: syn::File = syn::parse2(extra_tokens).unwrap();
    let extra_tokens = prettyplease::unparse(&extra_tokens);
    nodes_rs.push('\n');
    nodes_rs.push_str(&extra_tokens);
    nodes_rs.push('\n');

    fs::write(out_dir.join("nodes.rs"), nodes_rs).unwrap();
}
