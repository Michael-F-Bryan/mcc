use std::path::PathBuf;
use std::{env, fs};
use type_sitter_gen::generate_nodes;

fn main() {
    // Common setup. Same as before
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    println!("cargo::rerun-if-changed=build.rs");

    println!("cargo::rerun-if-changed=tree-sitter-c");

    // To generate nodes
    let nodes_rs = generate_nodes(tree_sitter_c::NODE_TYPES)
        .unwrap()
        .into_string();
    fs::write(out_dir.join("nodes.rs"), nodes_rs).unwrap();
}
