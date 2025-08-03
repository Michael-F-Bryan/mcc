use std::{ffi::OsString, path::PathBuf, process::Command};

use target_lexicon::{Architecture, OperatingSystem, Triple};

use crate::{CommandError, Db};

/// Turn an assembly file into object code.
#[tracing::instrument(level = "info", skip_all)]
#[salsa::tracked]
pub fn assemble_and_link(
    _db: &dyn Db,
    cc: OsString,
    assembly: PathBuf,
    dest: PathBuf,
    target: Triple,
) -> Result<(), CommandError> {
    let mut cmd = Command::new(cc);
    cmd.arg("-c").arg(assembly).arg("-o").arg(dest);

    if matches!(target.operating_system, OperatingSystem::Darwin(_))
        && !matches!(target.architecture, Architecture::Aarch64(_))
    {
        // Note: Make sure we cross-compile to x86 on MacOS
        cmd.arg("-target").arg(target.to_string());
    }

    crate::cmd::run_cmd(&mut cmd)?;

    Ok(())
}
