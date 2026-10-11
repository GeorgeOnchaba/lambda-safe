use lambda_safe_compiler::{Compiler, CompilerConfig, SourceFormat};
use lambda_safe_model::{CompileStatus, Counterexample, SolverIdentity};
use lambda_safe_solver::{DecisionProcedure, DecisionResult, SolverError, SolverQuery};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug)]
struct UnknownSolver;

impl DecisionProcedure for UnknownSolver {
    fn identity(&self) -> SolverIdentity {
        SolverIdentity {
            name: "unknown-test-double".to_owned(),
            version: "1".to_owned(),
            theory: "QF_LIA".to_owned(),
            options: BTreeMap::new(),
        }
    }

    fn solve(
        &self,
        _query: &SolverQuery,
        _timeout: Duration,
    ) -> Result<DecisionResult, SolverError> {
        Ok(DecisionResult::Unknown(
            "deliberate test outcome".to_owned(),
        ))
    }
}

#[derive(Debug)]
struct InvalidIdentitySolver;

impl DecisionProcedure for InvalidIdentitySolver {
    fn identity(&self) -> SolverIdentity {
        SolverIdentity {
            name: "invalid-test-double".to_owned(),
            version: "1".to_owned(),
            theory: "QF_NIA".to_owned(),
            options: BTreeMap::new(),
        }
    }

    fn solve(
        &self,
        _query: &SolverQuery,
        _timeout: Duration,
    ) -> Result<DecisionResult, SolverError> {
        Ok(DecisionResult::Unknown(
            "invalid identity must be rejected before solving".to_owned(),
        ))
    }
}

fn valid_source() -> &'static str {
    r#"lambda_safe "0.1";
contract adversarial version "1" {
  unit count;
  input x: int<count> authority control_plane range [0, 1];
  assume policy hard: x <= x;
  invariant reflexive hard: x <= x;
}"#
}

#[test]
fn solver_unknown_never_emits_certificate() {
    let compiler = Compiler::new(Arc::new(UnknownSolver), CompilerConfig::default());
    let result = compiler.compile(valid_source(), SourceFormat::Lambda);
    assert_eq!(result.status, CompileStatus::Indeterminate);
    assert!(result.artifacts.is_none());
}

#[test]
fn unsupported_solver_identity_never_reaches_discharge() {
    let compiler = Compiler::new(Arc::new(InvalidIdentitySolver), CompilerConfig::default());
    let result = compiler.compile(valid_source(), SourceFormat::Lambda);
    assert_eq!(result.status, CompileStatus::Indeterminate);
    assert!(result.verification.is_empty());
    assert!(result.artifacts.is_none());
}

#[test]
fn oversized_source_fails_before_parsing() {
    let config = CompilerConfig {
        max_source_bytes: 32,
        ..CompilerConfig::default()
    };
    let compiler = Compiler::new(Arc::new(UnknownSolver), config);
    let result = compiler.compile(valid_source(), SourceFormat::Lambda);
    assert_eq!(result.status, CompileStatus::Indeterminate);
    assert!(result.artifacts.is_none());
}

#[test]
fn unknown_json_field_is_rejected() {
    let source = r#"{
      "schema_version":"0.1",
      "language_version":"0.1",
      "contract_id":"x",
      "contract_version":"1",
      "smuggled_hard_claim":true
    }"#;
    let compiler = Compiler::new(Arc::new(UnknownSolver), CompilerConfig::default());
    let result = compiler.compile(source, SourceFormat::Json);
    assert_eq!(result.status, CompileStatus::MalformedContract);
    assert!(result.artifacts.is_none());
}

#[test]
fn certificate_tampering_is_detected() {
    let compiler = Compiler::new(
        Arc::new(lambda_safe_solver::FiniteModelSolver::default()),
        CompilerConfig::default(),
    );
    let mut artifacts = compiler
        .compile(valid_source(), SourceFormat::Lambda)
        .artifacts
        .expect("contract should certify");
    artifacts.manifest.contract_version = "substituted".to_owned();
    let result = compiler.verify(&artifacts.manifest, &artifacts.certificate);
    assert!(!result.valid);
    assert_eq!(result.checks.get("manifest_digest"), Some(&false));
}

#[test]
fn rewritten_hard_input_digest_is_detected_without_a_signature() {
    let compiler = Compiler::new(
        Arc::new(lambda_safe_solver::FiniteModelSolver::default()),
        CompilerConfig::default(),
    );
    let mut artifacts = compiler
        .compile(valid_source(), SourceFormat::Lambda)
        .artifacts
        .expect("contract should certify");
    artifacts.certificate.body.hard_input_digest = "00".repeat(32);
    artifacts.certificate.certificate_digest =
        lambda_safe_emit::digest_value(&artifacts.certificate.body)
            .expect("rewritten body should remain canonicalizable");
    let result = compiler.verify(&artifacts.manifest, &artifacts.certificate);
    assert!(!result.valid);
    assert_eq!(result.checks.get("hard_input_digest"), Some(&false));
}

#[test]
fn transition_relation_tampering_is_detected() {
    let compiler = Compiler::new(
        Arc::new(lambda_safe_solver::FiniteModelSolver::default()),
        CompilerConfig::default(),
    );
    let mut artifacts = compiler
        .compile(
            include_str!("../../../examples/valid/capacity.lambda"),
            SourceFormat::Lambda,
        )
        .artifacts
        .expect("contract should certify");
    let previous = artifacts.manifest.transitions[0]
        .relations
        .insert("fallback".to_owned(), "false".to_owned());
    assert!(previous.is_some());
    let result = compiler.verify(&artifacts.manifest, &artifacts.certificate);
    assert!(!result.valid);
    assert_eq!(
        result.checks.get("transition_relation_digests"),
        Some(&false)
    );
}

#[test]
fn solver_obligation_limit_fails_before_certification() {
    let config = CompilerConfig {
        max_solver_obligations: 0,
        ..CompilerConfig::default()
    };
    let compiler = Compiler::new(Arc::new(UnknownSolver), config);
    let result = compiler.compile(valid_source(), SourceFormat::Lambda);
    assert_eq!(result.status, CompileStatus::Indeterminate);
    assert!(result.verification.is_empty());
    assert!(result.artifacts.is_none());
}

#[test]
fn diagnostics_never_smuggle_a_counterexample_into_artifacts() {
    let _shape_guard: Option<Counterexample> = None;
    let source = include_str!("../../../examples/invalid/counterexample.lambda");
    let compiler = Compiler::new(
        Arc::new(lambda_safe_solver::FiniteModelSolver::default()),
        CompilerConfig::default(),
    );
    let result = compiler.compile(source, SourceFormat::Lambda);
    assert!(
        result
            .verification
            .iter()
            .any(|record| record.counterexample.is_some())
    );
    assert!(result.artifacts.is_none());
}

#[derive(Debug)]
struct DeadlineHonoringSolver;

impl DecisionProcedure for DeadlineHonoringSolver {
    fn identity(&self) -> SolverIdentity {
        SolverIdentity {
            name: "deadline-test-double".to_owned(),
            version: "1".to_owned(),
            theory: "QF_LIA".to_owned(),
            options: BTreeMap::new(),
        }
    }

    fn solve(
        &self,
        _query: &SolverQuery,
        timeout: Duration,
    ) -> Result<DecisionResult, SolverError> {
        let work = Duration::from_millis(15);
        if timeout < work {
            std::thread::sleep(timeout);
            return Err(SolverError::Timeout(timeout.as_millis() as u64));
        }
        std::thread::sleep(work);
        Ok(DecisionResult::Sat(Counterexample {
            assignments: BTreeMap::new(),
            raw_model: None,
        }))
    }
}

#[test]
fn whole_compilation_deadline_is_fail_closed() {
    let config = CompilerConfig {
        solver_timeout: Duration::from_secs(5),
        max_compile_time: Duration::from_millis(25),
        ..CompilerConfig::default()
    };
    let compiler = Compiler::new(Arc::new(DeadlineHonoringSolver), config);
    let started = std::time::Instant::now();
    let result = compiler.compile(valid_source(), SourceFormat::Lambda);
    assert_eq!(result.status, CompileStatus::Indeterminate);
    assert!(result.artifacts.is_none());
    assert!(started.elapsed() < Duration::from_millis(250));
}

#[test]
fn zero_compilation_budget_never_emits_artifacts() {
    let config = CompilerConfig {
        max_compile_time: Duration::ZERO,
        ..CompilerConfig::default()
    };
    let compiler = Compiler::new(
        Arc::new(lambda_safe_solver::FiniteModelSolver::default()),
        config,
    );
    let result = compiler.compile(valid_source(), SourceFormat::Lambda);
    assert_eq!(result.status, CompileStatus::Indeterminate);
    assert!(result.artifacts.is_none());
}

