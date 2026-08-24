//! Decision-procedure boundary for QF-LIA.
//!
//! The production adapter invokes Z3 through SMT-LIB 2.7. A deterministic
//! finite-domain solver is included for conformance tests and explicitly bounded
//! offline use. Neither adapter maps timeout, unknown or execution failure to
//! acceptance.

mod finite;
mod query;
mod smtlib;
mod z3;

pub use finite::{FiniteModelConfig, FiniteModelSolver};
pub use query::{DecisionProcedure, DecisionResult, SolverError, SolverQuery};
pub use smtlib::emit_smt2;
pub use z3::Z3ProcessSolver;

