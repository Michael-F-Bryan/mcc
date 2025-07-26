use std::{ffi::OsString, path::PathBuf, process::Command};

use crate::{CommandError, Db};

/// Turn an assembly file into object code.
#[salsa::tracked]
pub fn assemble_and_link(
    _db: &dyn Db,
    cc: OsString,
    assembly: PathBuf,
    dest: PathBuf,
) -> Result<(), CommandError> {
    let mut cmd = Command::new(cc);
    cmd.arg("-c").arg(assembly).arg("-o").arg(dest);

    crate::cmd::run_cmd(&mut cmd)?;

    Ok(())
}
