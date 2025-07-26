use crate::{Db, Text, types::Ast};

/// Compile a parsed C program into assembly.
#[salsa::tracked]
pub fn compile(_db: &dyn Db, _ast: Ast<'_>) -> Text {
    Text::default()
}
