# λ-Safe Compiler v0.1.0 — Hardening Release Candidate

## Scope preserved

This hardening pass does not change λ-Safe v0.1 language semantics or expand the certification claim beyond authority-bounded hard control-plane invariants in the supported QF-LIA fragment. Probabilistic and soft claims remain excluded from hard certification.

## Changes made

- Whole-compilation wall-clock budget: 30 seconds by default, configurable through the library, CLI and service (`LAMBDA_SAFE_MAX_COMPILE_MS`).
- Solver calls are capped by `min(per_obligation_timeout, remaining_compile_budget)`.
- Deadline exhaustion maps to `INDETERMINATE`; no artifact can be emitted.
- Stress tests: 512 hard invariants, obligation-limit rejection before solver work, and 16 concurrent deterministic compilations.
- Adversarial deadline tests: cumulative work cannot extend beyond the compilation budget without failing closed.
- Criterion scaling benchmarks: 1, 25, 100 and 500 hard invariants.
- New `artifact_verify` fuzz target for strict manifest/certificate deserialization and verification.
- New deterministic qualification script and explicit lockfile requirement for release verification.

## Execution status in this packaging environment

The source tree was inspected and statically transformed here, but this environment does not contain `cargo`, `rustc`, `z3` or Docker and has no network access to install them. Therefore this package does **not** claim that Rust compilation, tests, benchmarks, Z3 interoperability, fuzz campaigns or container checks were executed here.

`Cargo.lock` could not be truthfully generated without Cargo/dependency resolution. It is intentionally not fabricated. Before tagging the release, run `cargo generate-lockfile`, commit the resulting `Cargo.lock`, then execute `./scripts/qualification.sh` and the fuzz workflow. A release is qualified only after those executable checks are green.
