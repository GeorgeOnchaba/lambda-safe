use crate::SourceSpan;
use serde::{Deserialize, Serialize};

/// Diagnostic severity. Errors prevent certification; warnings never change
/// the hard result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    /// Informational context.
    Information,
    /// A non-fatal condition.
    Warning,
    /// A certification-blocking condition.
    Error,
}

/// Stable machine-readable diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    /// Stable code from `docs/diagnostics.md`.
    pub code: String,
    /// Severity.
    pub severity: DiagnosticSeverity,
    /// Concise human-readable explanation.
    pub message: String,
    /// Source location when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
    /// Optional declaration or claim identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// Actionable remediation guidance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

impl Diagnostic {
    /// Creates a certification-blocking diagnostic.
    #[must_use]
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: DiagnosticSeverity::Error,
            message: message.into(),
            span: None,
            subject: None,
            help: None,
        }
    }

    /// Associates the diagnostic with a source span.
    #[must_use]
    pub fn at(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }

    /// Associates the diagnostic with a declaration or claim.
    #[must_use]
    pub fn for_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Adds remediation guidance.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// Diagnostic code constants.
pub mod codes {
    /// Invalid source encoding or grammar.
    pub const INVALID_SOURCE: &str = "LSC-P001";
    /// Unsupported language/schema version.
    pub const VERSION_MISMATCH: &str = "LSC-P002";
    /// Duplicate declaration.
    pub const DUPLICATE_NAME: &str = "LSC-N001";
    /// Unknown referenced name.
    pub const UNKNOWN_NAME: &str = "LSC-N002";
    /// Expression type mismatch.
    pub const TYPE_MISMATCH: &str = "LSC-T001";
    /// Unit mismatch or invalid unit definition.
    pub const UNIT_MISMATCH: &str = "LSC-U001";
    /// Hard claim depends on insufficient authority.
    pub const AUTHORITY_VIOLATION: &str = "LSC-A001";
    /// Hard expression lies outside QF-LIA.
    pub const OUTSIDE_FRAGMENT: &str = "LSC-F001";
    /// Hard assumption context is infeasible.
    pub const INFEASIBLE_CONTEXT: &str = "LSC-V001";
    /// Hard invariant has a counterexample.
    pub const INVARIANT_VIOLATED: &str = "LSC-V002";
    /// Solver returned unknown, timed out or failed.
    pub const SOLVER_INDETERMINATE: &str = "LSC-S001";
    /// Degrade transition is malformed or non-preserving.
    pub const DEGRADE_VIOLATION: &str = "LSC-D001";
    /// Reconcile transition or strategy is invalid.
    pub const RECONCILE_VIOLATION: &str = "LSC-R001";
    /// Linear token was missing, duplicated or not consumed exactly once.
    pub const LINEAR_TOKEN_ERROR: &str = "LSC-L001";
    /// Canonicalization, digest or certificate construction failed.
    pub const ARTIFACT_FAILURE: &str = "LSC-C001";
    /// Certificate or signature verification failed.
    pub const CERTIFICATE_INVALID: &str = "LSC-C002";
    /// Contract exceeds a configured resource limit.
    pub const RESOURCE_LIMIT: &str = "LSC-X001";
}
