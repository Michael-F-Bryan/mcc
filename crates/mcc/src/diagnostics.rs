use codespan_reporting::diagnostic::{Label, Severity};
use salsa::Accumulator;

use crate::{
    Db, Text,
    types::{SourceFile, Span},
};

type CodespanDiagnostic = codespan_reporting::diagnostic::Diagnostic<SourceFile>;

/// A newtype wrapper around [`DiagnosticKind`] that is used to accumulate
/// errors as the compiler runs.
#[repr(transparent)]
#[salsa::accumulator]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic(pub DiagnosticKind);

impl Diagnostic {
    pub fn to_codespan(&self) -> CodespanDiagnostic {
        self.0.to_codespan()
    }
}

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

impl DiagnosticKind {
    pub fn to_codespan(&self) -> CodespanDiagnostic {
        match self {
            DiagnosticKind::Parse(e) => e.to_codespan(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
#[error("parse error: {msg}")]
pub struct ParseError {
    pub file: SourceFile,
    pub span: Span,
    pub msg: Text,
}

impl ParseError {
    pub fn accumulate(self, db: &dyn Db) {
        Diagnostic::from(self).accumulate(db);
    }

    pub fn to_codespan(&self) -> CodespanDiagnostic {
        CodespanDiagnostic::new(Severity::Error)
            .with_message(self.msg.to_string())
            .with_label(Label::primary(self.file, self.span))
    }
}

impl From<ParseError> for DiagnosticKind {
    fn from(value: ParseError) -> Self {
        DiagnosticKind::Parse(value)
    }
}
