# NiGeNje λ-Safe Compiler

The NiGeNje λ-Safe Compiler is a deterministic, fail-closed compiler and static
verifier for distributed control-plane contracts. It accepts a typed textual
language or strict JSON projection, verifies an authority-bounded QF-LIA hard
fragment, and emits a canonical hard manifest plus a λ-certificate only when
every required obligation succeeds.

This repository is the complete open-source v0.1 reference implementation. It
is independently usable and does not require the proprietary NiGeNje platform.

## Hard law

Certification requires all of the following:

1. The contract is structurally valid and version-compatible.
2. Every name resolves and every expression is well typed.
3. Integer operands use compatible, canonical units.
4. Every hard operand has control-plane or constant authority.
5. Every hard expression lies inside quantifier-free linear integer arithmetic.
6. The hard assumption context `Γ` is feasible: `SAT(Γ)`.
7. Every hard invariant `I` is valid: `UNSAT(Γ ∧ ¬I)`.
8. Every declared transition branch is feasible and preserves every named hard
   invariant.
9. Every linear token is consumed exactly once.
10. Canonicalization and artifact construction succeed.

Timeout, solver `unknown`, process failure, unsupported theory, missing
authority, inconsistent assumptions, counterexamples and internal errors never
produce a hard λ-certificate.

## What the certificate means

A λ-certificate records reproducible evidence that the declared hard
obligations were discharged under the exact canonical assumptions, compiler,
solver, strategy registry and theory recorded in the artifact.

It is not:

- a theorem about an unmodelled application;
- evidence that telemetry is truthful;
- a guarantee of future runtime behavior;
- a replacement for runtime enforcement or consensus;
- a proof-assistant-checked compiler-correctness theorem.

The compiler implementation and selected SMT solver remain in the trusted
computing base. See [Trust model](docs/trust-model.md).

## Supported v0.1 surface

- Boolean logic: `!`, `&&`, `||`, `=>`.
- Equality and ordering: `==`, `!=`, `<`, `<=`, `>`, `>=`.
- Linear integer arithmetic: `+`, `-`, and multiplication by a dimensionless
  compile-time integer.
- Named units with positive integer base-unit scaling.
- Boolean and bounded or unbounded integer inputs.
- `control_plane`, `trusted_report`, `predictive` and `unsupported` authority.
- Hard, trusted and soft assumptions/invariants.
- Primary/fallback degradation relations.
- Registered reconciliation strategies with declared merge relations.
- Present-state variables and `next(variable)` post-state references.
- Linear token declaration and exact-once consumption.

Quantifiers, nonlinear arithmetic, floating point, temporal logic, probability
as hard law, opaque oracles and application business semantics are outside the
v0.1 hard fragment.

## Workspace

| Crate | Responsibility |
|---|---|
| `lambda-safe-model` | Versioned contracts, expressions, diagnostics and artifact models |
| `lambda-safe-syntax` | Lexer, recursive-descent parser and JSON lowering |
| `lambda-safe-check` | Names, types, units, authority, QF-LIA, transitions and linear tokens |
| `lambda-safe-solver` | Solver trait, SMT-LIB emitter, Z3 adapter and bounded reference solver |
| `lambda-safe-emit` | RFC 8785/JCS canonicalization, digests, certificates and signatures |
| `lambda-safe-compiler` | Public library facade and fail-closed pipeline |
| `lambda-safe-cli` | `lambda-safe` command-line interface |
| `lambda-safe-service` | Stateless Axum HTTP service |

The repository also includes JSON Schemas, valid and invalid examples,
conformance/adversarial/property/stress tests, fuzz targets, scale benchmarks, CI, container
and Kubernetes resources, and a research traceability matrix.

## Prerequisites

- Rust 1.85 or newer with the 2024 edition.
- Z3 available as `z3` for the default production solver path.
- Docker or another OCI runtime for the container workflow.

The finite solver is exact only when every referenced integer has explicit
lower and upper bounds. It is intended for conformance, small offline contracts
and environments where Z3 is unavailable.

## Build and test

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --release
```

Run the complete local verification script:

```bash
./scripts/verify.sh
```

## Compile a contract

Using Z3:

```bash
cargo run -p lambda-safe-cli -- \
  --solver z3 \
  compile \
  --input examples/valid/capacity.lambda \
  --output artifacts/capacity
```

Using the bounded reference solver:

```bash
cargo run -p lambda-safe-cli -- \
  --solver finite \
  compile \
  --input examples/valid/capacity.lambda \
  --output artifacts/capacity
```

Successful compilation writes:

```text
artifacts/capacity/
├── compile-result.json
├── lambda-certificate.json
└── manifest.json
```

Rejected and indeterminate compilations print a structured result but do not
create hard artifacts.

The CLI treats an artifact directory as immutable: it refuses an existing
output path, writes all three files in a private staging directory and publishes
the directory with one rename only after every write succeeds.

## Other CLI operations

```bash
# Static gates only; no solver required
lambda-safe check --input examples/valid/capacity.lambda

# Verify digests and an optional Ed25519 signature
lambda-safe verify \
  --manifest artifacts/capacity/manifest.json \
  --certificate artifacts/capacity/lambda-certificate.json

# Print implementation/solver identity
lambda-safe --solver z3 info

# Print a bundled schema
lambda-safe schema contract
```

CLI exit codes are stable: `0` success, `2` malformed/unsupported/authority,
`3` counterexample or infeasible context, `4` indeterminate, `5` invalid
artifact and `70` execution/configuration failure.

## HTTP service

```bash
cargo run -p lambda-safe-service
```

Endpoints:

- `GET /healthz`
- `GET /readyz`
- `GET /v1/info`
- `POST /v1/check`
- `POST /v1/compile`
- `POST /v1/certificates/verify`

Example request:

```json
{
  "source": {
    "format": "lambda",
    "content": "lambda_safe \"0.1\"; contract demo version \"1\" { unit count; input x: int<count> authority control_plane range [0, 2]; assume a hard: x <= x; invariant i hard: x <= x; }"
  }
}
```

Configuration is described in [HTTP API](docs/api.md).

Compilation is resource bounded: input size, expression nesting/nodes,
declarations, generated obligations, concurrent HTTP requests, finite-domain
states, each solver call, and the complete compilation wall-clock budget all have explicit limits.
The default whole-compilation budget is 30 seconds (`LAMBDA_SAFE_MAX_COMPILE_MS=30000`),
so many individually legal solver obligations cannot accumulate into an unbounded worker hold.

## Optional certificate signing

Create a file containing exactly 64 hexadecimal characters representing a
32-byte Ed25519 secret key, restrict its filesystem permissions, then use:

```bash
lambda-safe --solver z3 compile \
  --input contract.lambda \
  --output artifacts/contract \
  --signing-key-file /secure/path/lambda-safe.key
```

Signing authenticates the certificate digest. It does not strengthen or alter
the logical result. The service reads the same key through
`LAMBDA_SAFE_SIGNING_KEY_FILE`. Production key generation and custody should use
an approved KMS or HSM workflow.

## Research artifact

The repository supports the MSc research concept *Design, Implementation and
Evaluation of a λ-Safe Compiler for Decidable Invariant Certification in
Distributed Control Planes*. Implementation claims, research objectives and
tests are mapped in [Research traceability](docs/research-traceability.md).
The generated 120-case matrix is a reproducible implementation corpus; formal
independent adjudication remains a research procedure and is not falsely
claimed by this software release.

## Open-source boundary

All soundness-critical semantics, parsers, checks, solver interfaces, hard
artifacts, certificate verification, CLI and service code are open. A future
commercial edition may add hosting, tenancy, enterprise integrations, key
management and governance workflows, but cannot redefine hard eligibility or
λ-certificate meaning.

## Security and responsible disclosure

Read [SECURITY.md](SECURITY.md) before deploying the service or reporting a
vulnerability. The service is stateless, applies source/body and expression
limits, performs no remote fetches and fails readiness when its configured Z3
binary cannot be discovered.

## Licence

Apache License 2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE). The licence does
not grant rights to NiGeNje trademarks.
