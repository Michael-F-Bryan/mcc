use crate::{Db, Text, types::Ast};

/// Compile a parsed C program into assembly.
#[salsa::tracked]
pub fn compile(db: &dyn Db, ast: Ast<'_>) -> Text {
    Text::default()
}
