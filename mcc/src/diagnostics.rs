//! Diagnostic reporting.

use codespan_reporting::{Diagnostic, Label, Severity};
use heapsize::HeapSizeOf;
use std::mem;

/// A collection of zero or more [`codespan_reporting::Diagnostic`] messages.
#[derive(Debug, Default, Clone)]
pub struct Diagnostics {
    diags: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Diagnostics {
        Diagnostics::default()
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diags
    }

    pub fn add(&mut self, diag: Diagnostic) {
        self.diags.push(diag);
    }

    /// How many [`Diagnostic`]s are this severe or greater?
    pub fn diagnostics_more_severe_than(&self, severity: Severity) -> usize {
        self.diags.iter().filter(|d| d.severity >= severity).count()
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics_more_severe_than(Severity::Error) > 0
    }

    pub fn has_warnings(&self) -> bool {
        self.diagnostics_more_severe_than(Severity::Warning) > 0
    }
}

fn diag_memory_usage(diag: &Diagnostic) -> usize {
    let &Diagnostic {
        severity: _,
        ref message,
        ref labels,
    } = diag;

    mem::size_of::<Diagnostic>()
        + message.heap_size_of_children()
        + labels.iter().map(label_memory_usage).sum::<usize>()
}

fn label_memory_usage(label: &Label) -> usize {
    let &Label {
        span: _,
        ref message,
        style: _,
    } = label;

    mem::size_of::<Label>() + message.heap_size_of_children()
}

impl HeapSizeOf for Diagnostics {
    fn heap_size_of_children(&self) -> usize {
        self.diags.iter().map(diag_memory_usage).sum()
    }
}

impl<'a> HeapSizeOf for &'a mut Diagnostics {
    fn heap_size_of_children(&self) -> usize {
        0
    }
}