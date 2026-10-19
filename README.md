# NiGeNje λ-Safe Compiler

The NiGeNje λ-Safe Compiler is a deterministic, fail-closed compiler and static
verifier for distributed control-plane contracts. It accepts a typed textual
language or strict JSON projection, verifies an authority-bounded QF-LIA hard
fragment, and emits a canonical hard manifest plus a λ-certificate only when
every required obligation succeeds.

This repository is the complete open-source v0.1 reference implementation. It
is independently usable and does not require the proprietary NiGeNje platform.

## One-command UI + backend

The official local stack composes this repository's compiler service with the
separate open-source UI repository
[GeorgeOnchaba/lambda-safe-fe](https://github.com/GeorgeOnchaba/lambda-safe-fe).

### Prerequisites

- Docker Engine and the Docker Compose plugin.
- Git.
- OpenSSL for a high-entropy local service token (or use the PowerShell recipe below).
- No hosted compiler API, hosted model, cloud account, or enterprise service.

### Start

Clone the two public repositories as sibling directories:

```bash
mkdir lambda-safe-stack && cd lambda-safe-stack
git clone https://github.com/GeorgeOnchaba/lambda-safe.git
git clone https://github.com/GeorgeOnchaba/lambda-safe-fe.git
cd lambda-safe
mkdir -p secrets
openssl rand -hex 32 > secrets/lambda-safe-api-token
chmod 600 secrets/lambda-safe-api-token
```

PowerShell alternative for token creation:

```powershell
New-Item -ItemType Directory -Force secrets | Out-Null
$bytes = New-Object byte[] 32
[System.Security.Cryptography.RandomNumberGenerator]::Fill($bytes)
$token = [Convert]::ToHexString($bytes).ToLowerInvariant()
[System.IO.File]::WriteAllText((Join-Path (Get-Location) 'secrets/lambda-safe-api-token'), $token)
```

Build and start both services using **one Compose command**:

```bash
docker compose up --build
```

Open http://localhost:3000. The backend is not published on the host; it can
only be reached over the private Compose network by the UI proxy. Stop with
`Ctrl+C` or `docker compose down`.

Compose expects the frontend repository at `../lambda-safe-fe` relative to
this repository. The UI proxy reads the token file and forwards it only to the
internal compiler API.

### What the UI supports

- Compiler health and identity display.
- λ-language, JSON projection and auto-detect request formats.
- Static checking and solver-backed compilation.
- Valid and failing sample contracts.
- Structured diagnostics and complete API response inspection.
- Manifest and λ-certificate views and JSON downloads.
- Calling the certificate/manifest integrity verification endpoint.

The UI is an API client. It does not reimplement or weaken compiler certification.

## Hard law

Certification requires all of the following:

1. The contract is structurally valid and version-compatible.
2. Every name resolves and every expression is well typed.
3. Integer operands use compatible, canonical units.
4. Every hard operand has control-plane or constant authority.
5. Every hard expression lies inside quantifier-free linear integer arithmetic.
6. The hard assumption context `Γ` is feasible: `SAT(Γ)`.
7. Every hard invariant `I` is valid: `UNSAT(Γ ∧ ¬I)`.
8. Every declared transition branch is feasible and preserves every named hard invariant.
9. Every linear token is consumed exactly once.
10. Canonicalization and artifact construction succeed.

Timeout, solver `unknown`, process failure, unsupported theory, missing
authority, inconsistent assumptions, counterexamples and internal errors never
produce a hard λ-certificate.

## What the certificate means

A λ-certificate records reproducible evidence that the declared hard
obligations were discharged under the exact canonical assumptions, compiler,
solver, strategy registry and theory recorded in the artifact.

It is not a theorem about an unmodelled application, evidence that telemetry is
truthful, a guarantee of future runtime behavior, a replacement for runtime
enforcement or consensus, or a proof-assistant-checked compiler-correctness
theorem. The compiler implementation and selected SMT solver remain in the
trusted computing base. See [Trust model](docs/trust-model.md).

## Supported v0.1 surface

- Boolean logic: `!`, `&&`, `||`, `=>`.
- Equality and ordering: `==`, `!=`, `<`, `<=`, `>`, `>=`.
- Linear integer arithmetic: `+`, `-`, and multiplication by a dimensionless compile-time integer.
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
| `lambda-safe-cli` | Command-line interface |
| `lambda-safe-service` | Stateless Axum HTTP service |

## Build and test

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --workspace --release --locked
./scripts/verify.sh
```

## HTTP API

- `GET /healthz`
- `GET /readyz`
- `GET /v1/info`
- `POST /v1/check`
- `POST /v1/compile`
- `POST /v1/certificates/verify`

The API is stateless and applies source/body and expression limits. Read
[HTTP API](docs/api.md) and [SECURITY.md](SECURITY.md) before deployment.

## Open-source boundary

The free repository includes soundness-critical language semantics, parser,
checks, solver interfaces, canonical hard artifacts, certificate verification,
CLI, HTTP service, schemas, tests, docs and the local UI client. The UI is MIT;
compiler core is Apache-2.0. The licenses do not grant rights to NiGeNje
trademarks.

A future commercial edition may add hosting, tenancy, enterprise integrations,
key management and governance workflows, but cannot redefine hard eligibility
or λ-certificate meaning. This boundary is a repository policy, not a guarantee
that private code can never leak: contributions still require review and secret
and dependency scanning.
