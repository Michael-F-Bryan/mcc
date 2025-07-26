use std::{
    ffi::OsString,
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::{Db, Text, types::PreprocessorInput};

/// Run the C preprocessor on the given input.
#[salsa::tracked]
pub fn preprocess(db: &dyn Db, input: PreprocessorInput) -> Result<Text, PreprocessorError> {
    let src = input.src(db);
    let path = src.path(db);
    let cc = input.cc(db);

    let mut cmd = Command::new(&cc);
    cmd.arg("-E")
        .arg("-P")
        .arg(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let Ok(output) = cmd.output() else {
        return Err(PreprocessorError {
            cc: input.cc(db),
            src: path.clone(),
            message: format!(
                "Failed to start \"{}\", is it installed?",
                cc.to_string_lossy()
            )
            .into(),
        });
    };

    if !output.status.success() {
        return Err(PreprocessorError {
            cc: input.cc(db),
            src: path.clone(),
            message: String::from_utf8_lossy(&output.stderr).into(),
        });
    }

    Ok(Text::from(String::from_utf8_lossy(&output.stdout)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreprocessorError {
    pub cc: OsString,
    pub src: PathBuf,
    pub message: Text,
}

impl std::error::Error for PreprocessorError {}

impl std::fmt::Display for PreprocessorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let PreprocessorError { cc, src, message } = self;
        let cc = cc.to_string_lossy();
        let src = src.display();
        let message = message.trim();
        write!(f, "preprocessing \"{src}\"with \"{cc}\" failed: {message}")
    }
}
