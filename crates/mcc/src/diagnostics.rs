use miette::{NamedSource, SourceSpan};
use salsa::Accumulator;

use crate::{Db, Text};

/// A newtype wrapper around [`DiagnosticKind`] that is used to accumulate
/// errors as the compiler runs.
#[repr(transparent)]
#[salsa::accumulator]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic(pub DiagnosticKind);

impl<K: Into<DiagnosticKind>> From<K> for Diagnostic {
    fn from(k: K) -> Self {
        Diagnostic(k.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
pub enum DiagnosticKind {
    #[error(transparent)]
    Parse(ParseError),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error, miette::Diagnostic)]
#[error("parse error: {msg}")]
#[diagnostic(severity(error))]
pub struct ParseError {
    #[source_code]
    pub src: NamedSource<Text>,
    pub msg: Text,
    #[label]
    pub span: SourceSpan,
}

impl ParseError {
    pub fn accumulate(self, db: &dyn Db) {
        Diagnostic::from(self).accumulate(db);
    }
}

impl From<ParseError> for DiagnosticKind {
    fn from(value: ParseError) -> Self {
        DiagnosticKind::Parse(value)
    }
}
