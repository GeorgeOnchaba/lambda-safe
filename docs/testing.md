# Testing Strategy

## Test layers

| Layer | Purpose |
|---|---|
| Unit | Parser precedence, typing, normalization, solver encoding, canonicalization |
| Conformance | 120 generated bounded QF-LIA contracts with expected outcomes |
| Adversarial | Fail-closed, deadline, tamper and non-elevation attacks |
| Integration | End-to-end text/JSON compilation and artifact verification |
| Property | Parser robustness and deterministic behavior over generated inputs |
| Fuzz | Arbitrary bytes into both source formats, the compile facade and artifact verification |
| Benchmark | Scale end-to-end compilation across 1/25/100/500 hard invariants |
| Service | Routing, body handling and status mapping |

## Required release commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p lambda-safe-compiler --test conformance --locked
cargo test -p lambda-safe-compiler --test adversarial --locked
cargo test -p lambda-safe-compiler --test stress --locked
cargo build --workspace --release --locked
./scripts/smoke-test.sh target/release/lambda-safe
```

The release workflow runs these checks on Linux. Fuzzing is scheduled separately
because it is duration based. A release record must distinguish checks actually
executed from checks merely defined.

## Oracle discipline

Finite-solver conformance cases have explicit bounded domains, making exhaustive
enumeration exact. Z3 interoperability tests should compare status and witness
validity rather than solver-specific model formatting. No test may treat
`unknown` as a successful proof.

## Adding semantics

Every new operator or strategy needs:

1. accepted syntax and JSON cases;
2. type/unit/authority rejections;
3. a satisfiable and unsatisfiable obligation;
4. timeout/unknown behavior;
5. canonical artifact stability;
6. an adversarial boundary case.

## Stress qualification

The stress suite includes a 512-invariant supported contract, obligation-limit fail-closed behavior, 16-way parallel deterministic compilation, and a compilation-wide deadline adversarial test. Criterion reports latency distributions for generated contracts at 1, 25, 100 and 500 hard invariants. `scripts/qualification.sh` runs the deterministic qualification surface; duration-based fuzzing remains a separate workflow.
