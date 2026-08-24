#![no_main]

use lambda_safe_compiler::{Compiler, CompilerConfig, SourceFormat};
use lambda_safe_solver::{FiniteModelConfig, FiniteModelSolver};
use libfuzzer_sys::fuzz_target;
use std::sync::Arc;

fuzz_target!(|data: &[u8]| {
    if let Ok(source) = std::str::from_utf8(data) {
        let config = CompilerConfig {
            max_source_bytes: 65_536,
            ..CompilerConfig::default()
        };
        let solver = FiniteModelSolver::new(FiniteModelConfig {
            max_variables: 32,
            max_values_per_integer: 64,
            max_assignments: 4_096,
        });
        let compiler = Compiler::new(Arc::new(solver), config);
        let _ = compiler.compile(source, SourceFormat::Lambda);
    }
});
