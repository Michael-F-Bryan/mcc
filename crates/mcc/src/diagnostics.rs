use std::ops::Deref;

use salsa::Accumulator;

use crate::types::SourceFile;

pub type Diagnostic = codespan_reporting::diagnostic::Diagnostic<SourceFile>;

pub trait DiagnosticExt {
    fn accumulate(self, db: &dyn crate::Db);
}

impl DiagnosticExt for Diagnostic {
    fn accumulate(self, db: &dyn crate::Db) {
        Diagnostics(self).accumulate(db);
    }
}

/// A newtype wrapper around [`Diagnostic`] that is used to accumulate errors as
/// the compiler runs.
#[repr(transparent)]
#[salsa::accumulator]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostics(pub Diagnostic);

impl From<Diagnostic> for Diagnostics {
    fn from(diagnostic: Diagnostic) -> Self {
        Diagnostics(diagnostic)
    }
}

impl Deref for Diagnostics {
    type Target = Diagnostic;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
