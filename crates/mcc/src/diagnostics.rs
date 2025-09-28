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

/// Declarative macro for defining error codes in a hierarchical structure.
///
/// # Example
/// ```rust
/// mcc::codes! {
///   parse {
///     /// The parser encountered an unexpected token.
///     const UNEXPECTED: &str = "unexpected";
///   }
///   types {
///     /// This part of the type checker isn't implemented.
///     const UNIMPLEMENTED: &str = "unimplemented";
///   }
/// }
/// ```
#[macro_export]
macro_rules! codes {
    // Base case: no more modules to process
    () => {};

    // Handle a module with constants
    (
        $(#[$mod_doc:meta])*
        $module:ident {
            $($(#[$doc:meta])* const $const_name:ident: &str = $value:expr;)*
        }
        $($rest:tt)*
    ) => {
        $( #[$mod_doc] )*
        pub mod $module {
            $(
                $(#[$doc])*
                pub const $const_name: &str = concat!(stringify!($module), "::", $value);
            )*
        }
        $crate::codes!($($rest)*);
    };

    // Handle nested modules
    (
        $(#[$mod_doc:meta])*
        $module:ident {
            $($nested:tt)*
        }
        $($rest:tt)*
    ) => {
        $( #[$mod_doc] )*
        pub mod $module {
            $crate::codes!($($nested)*);
        }
        $crate::codes!($($rest)*);
    };
}

/// Pre-defined error codes for common compiler diagnostics.
pub mod codes {
    codes! {
        /// Errors that can occur during parsing.
        parse {
            /// The parser encountered an unexpected token.
            const UNEXPECTED_TOKEN: &str = "unexpected_token";
            /// The parser expected a token but found none.
            const MISSING_TOKEN: &str = "missing_token";
        }

        /// Errors that can occur during type checking.
        type_check {
            /// This part of the type checker isn't implemented.
            const UNIMPLEMENTED: &str = "unimplemented";
        }
    }
}
