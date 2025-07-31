use std::{ffi::OsString, path::PathBuf, process::Command};

use target_lexicon::Triple;

use crate::{CommandError, Db};

/// Turn an assembly file into object code.
#[salsa::tracked]
pub fn assemble_and_link(
    _db: &dyn Db,
    cc: OsString,
    assembly: PathBuf,
    dest: PathBuf,
    target: Triple,
) -> Result<(), CommandError> {
    let mut cmd = Command::new(cc);
    cmd.arg("-c")
        .arg(assembly)
        .arg("-o")
        .arg(dest)
        .arg("-target")
        .arg(target.to_string());

    crate::cmd::run_cmd(&mut cmd)?;

    Ok(())
}
