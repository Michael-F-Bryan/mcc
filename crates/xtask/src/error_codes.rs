use std::{collections::BTreeMap, path::PathBuf, sync::LazyLock};

use anyhow::Context as _;
use clap::Parser;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::ROOT_DIR;

static MCC_DIR: LazyLock<PathBuf> = LazyLock::new(|| ROOT_DIR.join("crates/mcc"));
static ERROR_CODES_YAML: LazyLock<PathBuf> = LazyLock::new(|| MCC_DIR.join("error-codes.yaml"));
static CODES_RS: LazyLock<PathBuf> = LazyLock::new(|| MCC_DIR.join("src/codes.rs"));

#[derive(Debug, Parser, Clone, PartialEq)]
pub struct ErrorCodes {
    #[clap(short, long, default_value = CODES_RS.as_os_str())]
    output: PathBuf,
    #[clap(default_value = ERROR_CODES_YAML.as_os_str())]
    input: PathBuf,
}

impl ErrorCodes {
    pub fn run(self) -> anyhow::Result<()> {
        let ErrorCodes { output, input } = self;

        let src = std::fs::read_to_string(&input)
            .with_context(|| format!("reading \"{}\"", input.display()))?;
        let root_namespace: BTreeMap<String, Value> = serde_yaml::from_str(&src)
            .with_context(|| format!("parsing \"{}\"", input.display()))?;

        let tokens = generate_codes_rs(root_namespace).to_token_stream();
        crate::ensure_file_contents(&output, tokens, "error_codes");

        Ok(())
    }
}

fn generate_codes_rs(root_namespace: BTreeMap<String, Value>) -> impl ToTokens {
    let mut segments = Vec::new();
    let (tokens, error_codes) = generate_namespace(&mut segments, &root_namespace);

    quote! {
        //! Common error codes used across the compiler.
        #![allow(non_upper_case_globals)]
        use codespan_reporting::diagnostic::Severity;

        #[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
        pub struct ErrorCode {
            pub segments: &'static [&'static str],
            pub severity: Severity,
            pub description: &'static str,
        }

        impl std::fmt::Display for ErrorCode {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                for (i, segment) in self.segments.iter().enumerate() {
                    if i > 0 {
                        write!(f, "::")?;
                    }
                    write!(f, "{}", segment)?;
                }
                Ok(())
            }
        }

        /// All error codes.
        pub const ALL: &[ErrorCode] = &[ #(#error_codes,)* ];

        /// The error codes definition, as YAML.
        pub const DEFINITION: &str = include_str!(
            concat!(env!("CARGO_MANIFEST_DIR"), "/error-codes.yaml")
        );

        #tokens
    }
}

fn generate_namespace<'a>(
    segments: &mut Vec<&'a str>,
    namespace: &'a BTreeMap<String, Value>,
) -> (TokenStream, Vec<TokenStream>) {
    let mut tokens = TokenStream::new();
    let mut error_codes = Vec::new();

    for (name, value) in namespace {
        match value {
            Value::ErrorCode(error_code) => {
                error_codes.push(quote::format_ident!("{name}").to_token_stream());
                tokens.extend(generate_error_code(segments, name, error_code));
            }
            Value::Namespace(namespace) => {
                segments.push(name);
                let (toks, child_codes) = generate_namespace(&mut *segments, namespace);
                let ident = quote::format_ident!("{name}");
                for child in child_codes {
                    let new_code = quote!(#ident::#child);
                    error_codes.push(new_code);
                }
                tokens.extend(quote! {
                    pub mod #ident {
                        use super::*;
                        #toks
                    }
                });
                segments.pop();
            }
        }
    }

    (tokens, error_codes)
}

fn generate_error_code(segments: &[&str], name: &str, error_code: &ErrorCode) -> TokenStream {
    let ErrorCode {
        severity,
        description,
    } = error_code;
    let ident = quote::format_ident!("{name}");

    quote::quote! {
        #[doc = #description]
        pub const #ident: ErrorCode = ErrorCode {
            segments: &[#(#segments,)* #name],
            severity: #severity,
            description: #description,
        };
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum Value {
    ErrorCode(ErrorCode),
    Namespace(BTreeMap<String, Value>),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct ErrorCode {
    severity: Severity,
    description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum Severity {
    Bug,
    Error,
    Warning,
    Note,
    Help,
}

impl ToTokens for Severity {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = match self {
            Severity::Bug => "Bug",
            Severity::Error => "Error",
            Severity::Warning => "Warning",
            Severity::Note => "Note",
            Severity::Help => "Help",
        };
        let variant = quote::format_ident!("{name}");
        tokens.extend(quote!(Severity::#variant));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_up_to_date() {
        let error_codes = ErrorCodes {
            output: CODES_RS.clone(),
            input: ERROR_CODES_YAML.clone(),
        };

        error_codes.run().unwrap();
    }
}
