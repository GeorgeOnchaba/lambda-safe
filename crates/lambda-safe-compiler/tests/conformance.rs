use lambda_safe_compiler::{Compiler, CompilerConfig, SourceFormat};
use lambda_safe_model::CompileStatus;
use lambda_safe_solver::FiniteModelSolver;
use std::sync::Arc;

fn compiler() -> Compiler {
    Compiler::new(
        Arc::new(FiniteModelSolver::default()),
        CompilerConfig::default(),
    )
}

#[test]
fn curated_examples_have_declared_outcomes() {
    let cases = [
        (
            include_str!("../../../examples/valid/capacity.lambda"),
            SourceFormat::Lambda,
            CompileStatus::StaticallyCertified,
        ),
        (
            include_str!("../../../examples/valid/capacity.json"),
            SourceFormat::Json,
            CompileStatus::StaticallyCertified,
        ),
        (
            include_str!("../../../examples/valid/routing.lambda"),
            SourceFormat::Lambda,
            CompileStatus::StaticallyCertified,
        ),
        (
            include_str!("../../../examples/invalid/counterexample.lambda"),
            SourceFormat::Lambda,
            CompileStatus::InvariantViolated,
        ),
        (
            include_str!("../../../examples/invalid/infeasible.lambda"),
            SourceFormat::Lambda,
            CompileStatus::InfeasibleContext,
        ),
        (
            include_str!("../../../examples/invalid/authority-violation.lambda"),
            SourceFormat::Lambda,
            CompileStatus::AuthorityViolation,
        ),
        (
            include_str!("../../../examples/invalid/unit-mismatch.lambda"),
            SourceFormat::Lambda,
            CompileStatus::MalformedContract,
        ),
        (
            include_str!("../../../examples/invalid/nonlinear.lambda"),
            SourceFormat::Lambda,
            CompileStatus::Unsupported,
        ),
        (
            include_str!("../../../examples/invalid/transition-violation.lambda"),
            SourceFormat::Lambda,
            CompileStatus::InvariantViolated,
        ),
        (
            include_str!("../../../examples/invalid/linear-token.lambda"),
            SourceFormat::Lambda,
            CompileStatus::MalformedContract,
        ),
        (
            include_str!("../../../examples/invalid/transition-omits-invariant.lambda"),
            SourceFormat::Lambda,
            CompileStatus::Unsupported,
        ),
    ];
    for (source, format, expected) in cases {
        let result = compiler().compile(source, format);
        assert_eq!(result.status, expected, "unexpected result: {result:#?}");
        assert_eq!(result.artifacts.is_some(), expected.permits_certificate());
    }
}

#[test]
fn generated_conformance_matrix_contains_at_least_120_cases() {
    let mut executed = 0_usize;
    for index in 0..30 {
        let upper = 2 + index % 3;
        let valid = format!(
            r#"lambda_safe "0.1";
contract valid_{index} version "1" {{
  unit count;
  const limit: int<count> = {upper};
  input x: int<count> authority control_plane range [0, {upper}];
  assume policy hard: x <= limit;
  invariant bounded hard: x <= limit;
}}"#
        );
        assert_eq!(
            compiler().compile(&valid, SourceFormat::Lambda).status,
            CompileStatus::StaticallyCertified
        );
        executed += 1;

        let violated = format!(
            r#"lambda_safe "0.1";
contract violated_{index} version "1" {{
  unit count;
  const limit: int<count> = {upper};
  input x: int<count> authority control_plane range [0, {upper}];
  invariant too_small hard: x < limit;
}}"#
        );
        assert_eq!(
            compiler().compile(&violated, SourceFormat::Lambda).status,
            CompileStatus::InvariantViolated
        );
        executed += 1;

        let infeasible = format!(
            r#"lambda_safe "0.1";
contract infeasible_{index} version "1" {{
  unit count;
  const zero: int<count> = 0;
  input x: int<count> authority control_plane range [0, {upper}];
  assume impossible hard: x < zero;
  invariant nonnegative hard: x >= zero;
}}"#
        );
        assert_eq!(
            compiler().compile(&infeasible, SourceFormat::Lambda).status,
            CompileStatus::InfeasibleContext
        );
        executed += 1;

        let authority = format!(
            r#"lambda_safe "0.1";
contract authority_{index} version "1" {{
  unit count;
  const zero: int<count> = 0;
  input x: int<count> authority trusted_report range [0, {upper}];
  invariant asserted hard: x >= zero;
}}"#
        );
        assert_eq!(
            compiler().compile(&authority, SourceFormat::Lambda).status,
            CompileStatus::AuthorityViolation
        );
        executed += 1;
    }
    assert!(executed >= 120);
}

#[test]
fn canonical_artifacts_are_repeatable_across_100_compilations() {
    let source = include_str!("../../../examples/valid/routing.lambda");
    let baseline = compiler()
        .compile(source, SourceFormat::Lambda)
        .artifacts
        .expect("baseline should certify")
        .certificate
        .certificate_digest;
    for _ in 0..100 {
        let digest = compiler()
            .compile(source, SourceFormat::Lambda)
            .artifacts
            .expect("repeat should certify")
            .certificate
            .certificate_digest;
        assert_eq!(digest, baseline);
    }
}

#[test]
fn whitespace_and_comments_do_not_change_hard_digest() {
    let compact = r#"lambda_safe "0.1"; contract stable version "1" { unit count; input x: int<count> authority control_plane range [0, 2]; assume a hard: x <= x; invariant i hard: x <= x; }"#;
    let formatted = r#"
        // Equivalent hard input with different layout.
        lambda_safe "0.1";
        contract stable version "1" {
          unit count;
          input x: int<count> authority control_plane range [0, 2];
          assume a hard: x <= x;
          invariant i hard: x <= x;
        }
    "#;
    let first = compiler()
        .compile(compact, SourceFormat::Lambda)
        .artifacts
        .expect("compact source should certify")
        .certificate;
    let second = compiler()
        .compile(formatted, SourceFormat::Lambda)
        .artifacts
        .expect("formatted source should certify")
        .certificate;
    assert_eq!(first.certificate_digest, second.certificate_digest);
    assert_eq!(first.body.hard_input_digest, second.body.hard_input_digest);
}
