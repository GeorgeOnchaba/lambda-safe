use lambda_safe_compiler::{Compiler, CompilerConfig, SourceFormat};
use lambda_safe_model::CompileStatus;
use lambda_safe_solver::FiniteModelSolver;
use std::sync::Arc;

fn generated_contract(invariants: usize) -> String {
    let mut source = String::from(
        "lambda_safe \"0.1\";\ncontract generated_stress version \"1\" {\n  unit count;\n  const limit: int<count> = 2;\n  input x: int<count> authority control_plane range [0, 2];\n  assume bounded hard: x <= limit;\n",
    );
    for index in 0..invariants {
        source.push_str(&format!("  invariant inv_{index} hard: x <= limit;\n"));
    }
    source.push_str("}\n");
    source
}

fn compiler(config: CompilerConfig) -> Compiler {
    Compiler::new(Arc::new(FiniteModelSolver::default()), config)
}

#[test]
fn large_generated_contract_certifies_within_supported_bounds() {
    let source = generated_contract(512);
    let result = compiler(CompilerConfig::default()).compile(&source, SourceFormat::Lambda);
    assert_eq!(result.status, CompileStatus::StaticallyCertified);
    assert!(result.artifacts.is_some());
    assert_eq!(result.verification.len(), 513);
}

#[test]
fn obligation_boundary_rejects_before_any_solver_work() {
    let config = CompilerConfig {
        max_solver_obligations: 64,
        ..CompilerConfig::default()
    };
    let result = compiler(config).compile(&generated_contract(64), SourceFormat::Lambda);
    assert_eq!(result.status, CompileStatus::Indeterminate);
    assert!(result.verification.is_empty());
    assert!(result.artifacts.is_none());
}

#[test]
fn parallel_compilation_is_deterministic() {
    let source = Arc::new(generated_contract(64));
    let baseline = compiler(CompilerConfig::default())
        .compile(source.as_str(), SourceFormat::Lambda)
        .artifacts
        .expect("baseline should certify")
        .certificate
        .certificate_digest;

    let mut handles = Vec::new();
    for _ in 0..16 {
        let source = Arc::clone(&source);
        handles.push(std::thread::spawn(move || {
            compiler(CompilerConfig::default())
                .compile(source.as_str(), SourceFormat::Lambda)
                .artifacts
                .expect("parallel compile should certify")
                .certificate
                .certificate_digest
        }));
    }
    for handle in handles {
        assert_eq!(handle.join().expect("worker should not panic"), baseline);
    }
}
