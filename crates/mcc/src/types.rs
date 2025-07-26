use std::{ffi::OsString, ops::Deref, path::PathBuf};

use miette::NamedSource;

use crate::{Db, Text};

#[salsa::input]
pub struct PreprocessorInput {
    pub cc: OsString,
    #[returns(ref)]
    pub src: SourceFile,
}

#[salsa::input]
#[derive(Debug)]
pub struct SourceFile {
    #[returns(ref)]
    pub path: PathBuf,
    #[returns(ref)]
    pub contents: Text,
}

impl SourceFile {
    pub fn named_source(self, db: &dyn Db) -> NamedSource<Text> {
        NamedSource::new(
            self.path(db).display().to_string(),
            self.contents(db).clone(),
        )
    }
}

#[salsa::tracked]
pub struct Ast<'db> {
    pub tree: Tree,
}

#[derive(Debug, Clone)]
pub struct Tree(pub tree_sitter::Tree);

impl From<tree_sitter::Tree> for Tree {
    fn from(value: tree_sitter::Tree) -> Self {
        Tree(value)
    }
}

impl Deref for Tree {
    type Target = tree_sitter::Tree;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq for Tree {
    fn eq(&self, other: &Self) -> bool {
        self.0.root_node() == other.0.root_node()
    }
}

impl Eq for Tree {}

impl std::hash::Hash for Tree {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.root_node().hash(state);
    }
}
