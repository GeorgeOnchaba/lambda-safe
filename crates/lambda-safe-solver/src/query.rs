use lambda_safe_check::CoreSymbol;
use lambda_safe_model::{Counterexample, Expr, SolverIdentity};
use std::time::Duration;
use thiserror::Error;

/// One satisfiability query. Validity is encoded by adding the negated claim to
/// the assertion list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SolverQuery {
    /// Stable obligation id for diagnostics and evidence.
    pub id: String,
    /// Runtime declarations and their optional finite bounds.
    pub symbols: Vec<CoreSymbol>,
    /// Boolean assertions conjoined by the adapter.
    pub assertions: Vec<Expr>,
}

/// Safe decision result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecisionResult {
    /// A satisfying assignment exists.
    Sat(Counterexample),
    /// No satisfying assignment exists.
    Unsat,
    /// The adapter cannot provide a safe yes/no answer.
    Unknown(String),
}

/// Execution or protocol failure. Callers must treat every variant as
/// indeterminate and must not emit a hard certificate.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SolverError {
    /// Solver process could not be started or communicated with.
    #[error("solver I/O failure: {0}")]
    Io(String),
    /// Solver exceeded the configured deadline.
    #[error("solver timed out after {0} ms")]
    Timeout(u64),
    /// Solver output violated the expected protocol.
    #[error("invalid solver response: {0}")]
    Protocol(String),
    /// Expression could not be evaluated safely by an adapter.
    #[error("solver evaluation failure: {0}")]
    Evaluation(String),
}

/// Pluggable, fail-closed decision procedure.
pub trait DecisionProcedure: Send + Sync {
    /// Stable identity included in every λ-certificate.
    fn identity(&self) -> SolverIdentity;

    /// Decides satisfiability under a hard deadline.
    fn solve(&self, query: &SolverQuery, timeout: Duration) -> Result<DecisionResult, SolverError>;
}

