use std::{
    ffi::OsString,
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::{Db, Text, types::SourceFile};

/// Run the C preprocessor on the given input.
#[tracing::instrument(level = "info", skip_all)]
#[salsa::tracked]
pub fn preprocess(db: &dyn Db, cc: OsString, src: SourceFile) -> Result<Text, PreprocessorError> {
    let path = src.path(db);

    let mut cmd = Command::new(&cc);
    cmd.arg("-E")
        .arg("-P")
        .arg(path.as_str())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let Ok(output) = cmd.output() else {
        let msg = format!(
            "Failed to start \"{}\", is it installed?",
            cc.to_string_lossy()
        );
        return Err(PreprocessorError {
            cc,
            path: PathBuf::from(path.as_str()),
            message: msg.into(),
        });
    };

    if !output.status.success() {
        return Err(PreprocessorError {
            cc,
            path: PathBuf::from(path.as_str()),
            message: String::from_utf8_lossy(&output.stderr).into(),
        });
    }

    Ok(Text::from(String::from_utf8_lossy(&output.stdout)))
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("preprocessing \"{path}\" with \"{cc}\" failed: {message}", cc=cc.to_string_lossy())]
pub struct PreprocessorError {
    pub cc: OsString,
    pub path: PathBuf,
    pub message: Text,
}
