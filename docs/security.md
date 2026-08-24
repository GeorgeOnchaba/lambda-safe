# Security Model

## Protected property

The primary property is certificate non-forgery at the semantic boundary: a
v0.1 hard certificate must not be emitted unless the exact canonical hard model
passes every static gate and solver obligation. Availability and signing-key
custody are operational properties layered around that rule.

## Trust zones

| Zone | Trusted for | Not trusted for |
|---|---|---|
| Contract author | Declaring the intended model | Truth of claims or authority labels |
| Parser/checker | Correct lowering and eligibility | Runtime enforcement |
| Decision procedure | Correct SAT/UNSAT answers in the declared theory | Telemetry truth |
| Emitter | Canonical bytes, hashes and signatures | Model completeness |
| Consumer | Checking artifacts before use | Reinterpreting certificate semantics |

## Threats and controls

| Threat | Required control |
|---|---|
| Soft/trusted claim elevation | Explicit claim class plus hard-symbol projection |
| Nonlinear or unsupported escape | Structural expression gate before solving |
| Vacuous proof | Separate `SAT(Γ)` and transition-feasibility obligations |
| Solver ambiguity | `unknown`, timeout, parse error and process failure fail closed |
| Unit confusion | Canonical integer scaling and compatible-unit checks |
| Artifact substitution | Manifest/certificate SHA-256 binding and reconstructed hard-input digest |
| Partial or accidental artifact replacement | Immutable CLI output path and staged directory publication |
| Signature confusion | Ed25519 algorithm id and public key embedded in signature record |
| Resource exhaustion | Source, token, nesting, node, obligation, concurrency, per-solver, whole-compilation and finite-state limits |
| Ambiguous JSON member shadowing | Duplicate and unknown object members are rejected recursively |
| Command injection | Z3 path is an executable argument; source is sent through stdin |
| Nondeterminism | Sorted hard projection and JCS serialization |

## Solver process

The Z3 adapter uses `QF_LIA`, sets an internal timeout and applies an external
deadline. It does not interpolate contract strings into a shell command.
Production isolation should still restrict process count, CPU, memory, file
access and networking.

## Cryptography

SHA-256 binds canonical data. Optional Ed25519 authenticates the certificate
digest; it does not improve logical assurance. Verification recomputes both the
manifest digest and certificate body digest. Key generation, rotation,
revocation and identity binding are deliberately deployment concerns.

## Residual risks

- Defects in the implementation, dependencies or solver.
- A contract that omits a real-world behavior.
- Incorrect authority assignment at system integration points.
- Side channels and denial of service in a shared solver environment.
- Unicode confusables in free-form human strings; identifiers themselves are
  restricted to ASCII.
- No mechanized compiler-correctness or proof-assistant proof in v0.1.

### Whole-compilation deadline

Per-obligation timeouts are not sufficient against contracts that generate many expensive obligations. The compiler therefore starts a wall-clock deadline at the beginning of `compile`; every solver call receives the smaller of the configured per-obligation timeout and the remaining compilation budget. Exhaustion maps to `INDETERMINATE` and never emits artifacts.
