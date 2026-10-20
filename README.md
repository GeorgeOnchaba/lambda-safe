# NiGeNje λ-Safe Compiler

The NiGeNje λ-Safe Compiler is a deterministic, fail-closed compiler and static
verifier for distributed control-plane contracts. It accepts a typed textual
language or strict JSON projection, verifies an authority-bounded QF-LIA hard
fragment, and emits a canonical hard manifest plus a λ-certificate only when
every required obligation succeeds.

This repository is the complete open-source v0.1 reference implementation. It
is independently usable and does not require the proprietary NiGeNje platform.

## One-command UI + backend

The local stack runs this compiler service with the separate open-source UI
repository [GeorgeOnchaba/lambda-safe-fe](https://github.com/GeorgeOnchaba/lambda-safe-fe).
The repositories remain independent: Compose builds the backend from this
checkout and fetches the frontend from its own Git repository as a remote build
context. No monorepo or sibling frontend checkout is required.

### Prerequisites

- Docker Engine / Docker Desktop with Docker Compose v2.
- OpenSSL for a high-entropy local service token, or use the PowerShell recipe below.
- Internet access for the initial frontend build.
- No hosted compiler API, hosted model, cloud account, or enterprise service.

### Start

Clone only the backend repository and create a local API token:

```bash
git clone https://github.com/GeorgeOnchaba/lambda-safe.git
cd lambda-safe
mkdir -p secrets
openssl rand -hex 32 > secrets/lambda-safe-api-token
chmod 644 secrets/lambda-safe-api-token
docker compose up --build
```

PowerShell alternative for token creation:

```powershell
New-Item -ItemType Directory -Force secrets | Out-Null
$bytes = New-Object byte[] 32
[System.Security.Cryptography.RandomNumberGenerator]::Fill($bytes)
$token = [Convert]::ToHexString($bytes).ToLowerInvariant()
[System.IO.File]::WriteAllText((Join-Path (Get-Location) 'secrets/lambda-safe-api-token'), $token)
docker compose up --build
```

Open **http://localhost:3001**. The UI is published only on loopback
(`127.0.0.1`). The compiler is not published on a host port; it is reachable
only over the private Compose network through the UI proxy. The UI proxy reads
the same Docker secret and forwards the Bearer token server-side; the browser
never receives the token.

Stop with `Ctrl+C` or run `docker compose down`.

### Useful commands

```bash
# Run in the background
docker compose up --build -d

# Follow logs
docker compose logs -f compiler ui

# Inspect container state
docker compose ps

# Check UI health and the proxied compiler endpoints
curl -fsS http://localhost:3001/healthz
curl -fsS http://localhost:3001/readyz
curl -fsS http://localhost:3001/v1/info

# Stop and remove containers/network
docker compose down

# Rebuild without cached layers
docker compose build --no-cache
docker compose up
```

### Local security notes

- The token file under `secrets/` is ignored by Git and the backend Docker build context.
- Generate a different high-entropy token for each local environment.
- Both services run as UID 10001. On Linux, file-backed Compose secrets preserve host permissions, so the local demo token must be readable by that UID (`0644`). Keep the token out of version control; this demo setup is not a production secret-management pattern.
- Do not publish port 8080 or remove the private network boundary.
- This Compose configuration is for local development and demonstration, not a production deployment.
- The remote frontend build context currently tracks the frontend repository's `main` branch. For a reproducible release, pin it to a reviewed commit SHA after the complete stack has been tested.

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

