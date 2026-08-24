//! Textual λ-Safe and JSON-projection parsing.

mod json;
mod lexer;
mod parser;
mod strict_json;

pub use json::parse_json_projection;
pub use parser::{parse_expression, parse_lambda_source};
pub use strict_json::deserialize_json_strict;

use lambda_safe_model::{ContractDocument, Diagnostic};
use thiserror::Error;

/// Caller-selected input format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFormat {
    /// Detect JSON from the first non-whitespace byte; otherwise use λ-Safe text.
    Auto,
    /// Textual λ-Safe language.
    Lambda,
    /// JSON contract projection.
    Json,
}

/// Parsing failure with a stable compiler diagnostic.
#[derive(Clone, Debug, Error)]
#[error("{diagnostic}", diagnostic = .diagnostic.message)]
pub struct SyntaxError {
    /// Stable diagnostic.
    pub diagnostic: Box<Diagnostic>,
}

impl SyntaxError {
    pub(crate) fn new(diagnostic: Diagnostic) -> Self {
        Self {
            diagnostic: Box::new(diagnostic),
        }
    }
}

/// Parses a complete contract in the selected format.
pub fn parse_contract(source: &str, format: SourceFormat) -> Result<ContractDocument, SyntaxError> {
    let selected = match format {
        SourceFormat::Auto if source.trim_start().starts_with('{') => SourceFormat::Json,
        SourceFormat::Auto => SourceFormat::Lambda,
        explicit => explicit,
    };

    match selected {
        SourceFormat::Lambda => parse_lambda_source(source),
        SourceFormat::Json => parse_json_projection(source),
        SourceFormat::Auto => unreachable!("auto format is resolved before dispatch"),
    }
}
