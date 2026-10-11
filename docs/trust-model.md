# Trust Model

## Assets

- Correct classification of hard versus non-hard claims.
- Integrity and reproducibility of manifests and λ-certificates.
- Stable fail-closed behavior under malformed or adversarial input.
- Confidentiality of an optional signing key.
- Availability of the stateless compile service within configured limits.

## Trusted computing base

The v0.1 trusted computing base contains:

- the Rust compiler and target platform;
- this parser, checker, normalizer, pipeline and artifact implementation;
- the selected decision procedure and its configured version;
- cryptographic and canonicalization dependencies;
- the operator-controlled signing-key environment when signing is enabled;
- the correctness of declared control-plane authority and transition relations.

The certificate records the compiler and solver identity but does not remove
them from the trusted base.

## Untrusted inputs

- Contract source and JSON documents.
- Names, metadata and provenance labels.
- Trusted-report and predictive values.
- HTTP request bodies.
- Stored manifests/certificates presented for offline verification.

The compiler never fetches referenced URLs, executes contract code or loads
plugins from input.

## Authority boundary

`control_plane` means the contract declares the operand as governed by the
control-plane model. The compiler verifies logical relationships over this
declaration; it does not independently inspect the runtime producer.

`trusted_report` means an external producer asserts the value. The claim may be
reported to an operator but cannot enter a hard proof context. `predictive`
means an estimate. `unsupported` is rejected from hard use.

An authority label is not a remote attestation. Runtime provenance validation,
freshness and enforcement are integration responsibilities.

## Threats and controls

| Threat | Control |
|---|---|
| Inconsistent assumptions create vacuous truth | Separate `SAT(Γ)` gate |
| Trusted telemetry is relabelled hard | Operand-level authority gate |
| Nonlinear/undecidable formula reaches SMT | Typed AST plus QF-LIA fragment gate |
| Solver timeout is treated as success | `Unknown`/timeout/error are indeterminate |
| Unsatisfiable transition appears preserving | Separate branch-feasibility query |
| Soft metadata changes certificate | Hard-only normalized projection |
| Manifest/certificate substitution | Cross-digests and offline verification |
| Certificate forgery | Digests detect internal mutation; an optional Ed25519 signature is checked against its embedded public key but does not establish issuer identity |
| Parser/resource exhaustion | Source, declaration, expression, body, timeout and finite-state limits |
| Duplicate JSON keys create divergent interpretations | Recursive duplicate-member rejection before typed lowering |
| Cross-request leakage | Stateless compiler and immutable shared state |
| Runtime violates declared relation | Explicit non-guarantee; runtime conformance is outside compiler |

## Non-guarantees

The compiler does not guarantee availability of external infrastructure,
application correctness, business invariants, absence of races, telemetry
truth, implementation conformance to a transition relation, or future behavior.
It does not certify secrets and should not receive them in contract metadata.

## Deployment recommendations

- Pin the Rust toolchain, dependencies, container digest and Z3 version.
- Place the service behind authentication and network policy.
- Keep a reverse-proxy request deadline slightly above `LAMBDA_SAFE_MAX_COMPILE_MS`. The compiler itself enforces a whole-compilation wall-clock budget in addition to the per-obligation timeout.
- Use a read-only root filesystem and non-root user.
- Keep signing keys in KMS/HSM-backed storage; a mounted key file is a portable
  reference path, not the preferred enterprise custody mechanism.
- Archive source, hard artifacts, compiler image digest and solver identity.
- Use an external trust-aware verifier at the admission boundary. Core's
  offline `valid` result is integrity and binding only; it has no issuer trust
  roots, authorization policy or revocation data.
