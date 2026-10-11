#![no_main]

use lambda_safe_check::CoreSymbol;
use lambda_safe_model::Expr;
use lambda_safe_solver::{
    DecisionProcedure, FiniteModelConfig, FiniteModelSolver, SolverQuery, emit_smt2,
};
use lambda_safe_syntax::deserialize_json_strict;
use libfuzzer_sys::fuzz_target;
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FuzzQuery {
    id: String,
    symbols: Vec<CoreSymbol>,
    assertions: Vec<Expr>,
}

fuzz_target!(|data: &[u8]| {
    if let Ok(source) = std::str::from_utf8(data)
        && let Ok(input) = deserialize_json_strict::<FuzzQuery>(source)
    {
        let query = SolverQuery {
            id: input.id,
            symbols: input.symbols,
            assertions: input.assertions,
        };
        let _ = emit_smt2(&query, Duration::from_millis(25));
        let solver = FiniteModelSolver::new(FiniteModelConfig {
            max_variables: 16,
            max_values_per_integer: 32,
            max_assignments: 512,
        });
        let _ = solver.solve(&query, Duration::from_millis(25));
    }
});
