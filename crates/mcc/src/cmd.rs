use std::process::{Command, Output};

use crate::Text;

/// Run a command and return its output.
///
/// If the command fails, this function will return an error.
pub(crate) fn run_cmd(cmd: &mut Command) -> Result<Output, CommandError> {
    let output = cmd.output().map_err(|e| CommandError::StartFailed {
        cmd: format!("{cmd:?}").into(),
        error: e.to_string().into(),
    })?;

    if !output.status.success() {
        return Err(CommandError::CompletedUnsuccessfully {
            cmd: format!("{cmd:?}").into(),
            status: output.status.code().unwrap_or(1),
            stderr: String::from_utf8_lossy(&output.stderr).into(),
        });
    }

    Ok(output)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
pub enum CommandError {
    #[error("unable to start \"{cmd}\": {error:?}")]
    StartFailed { cmd: Text, error: Text },
    #[error("command \"{cmd}\" completed unsuccessfully: {status}")]
    CompletedUnsuccessfully {
        cmd: Text,
        status: i32,
        stderr: Text,
    },
}
