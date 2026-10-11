# Enterprise v1.0 Baseline Audit — λ-Safe Core

**Audit date:** 2026-10-12  
**Baseline:** *λ-Safe Enterprise v1.0 — Implementable Technical Specification*,
version 1.0, dated 2026-08-28  
**Scope:** Public `gee-cpu/lambda-safe` Core and the specification's Phase 0
Core repair and qualification gates.

## Decision

The current Core working tree builds, passes the workspace test suite, and
passes a finite-solver end-to-end artifact round-trip on Windows. The source
archive alarm in the baseline about the missing transition deadline is fixed
in the current code. The Core is suitable for continued development and
research use, with its documented QF-LIA and runtime limits.

This is not evidence that λ-Safe Enterprise v1.0 is implemented or ready for
production. The enterprise control plane, tenant isolation, trusted issuer
registry, durable evidence service, revocation, approvals, Kubernetes
admission, disaster recovery and operational qualification are outside this
public Core repository. Several Core release gates also remain unqualified
because this environment has no runnable Z3 or Docker engine and cannot write
to the Cargo registry cache needed by the fuzz workspace.

The baseline specification identifies its audited source archive by SHA-256
`4c0caf33e2a465a51827e85f739e2c69bb1d13926dd7fc280d13feb745c14188`. This
audit instead evaluates the repository tree available now: local base commit
`c7416e8f11b9fe4024b788f284cea3dbfe31df15`, whose tree
`8147d0f18b5a256de9b8a2469a6c3316e82230db` matched `origin/main` at audit
start, plus the uncommitted assurance changes in this working tree. The
repository `VERSION` is `0.1.0`; this is not the Enterprise artifact profile
1.0 or the Core `0.1.1` release proposed by the specification.

## Core audit findings

| Baseline reference | Result in current Core | Evidence and remaining limit |
|---|---|---|
| OS-001 — Transition deadline omission | **Fixed and regression-tested** | Every compiler solver call receives the shared deadline. The new `transition_feasibility_receives_whole_job_deadline` adversarial test exercises a transition-bearing JSON contract and checks for `INDETERMINATE` with no artifacts when the transition query runs out of job time. The valid transition example also certifies with the finite solver. |
| OS-002 — Missing reproducible lock | **Present** | `Cargo.lock` exists. Workspace tests, release builds, Docker builds and CI commands use `--locked`. |
| OS-003 — No executed qualification | **Partially addressed** | See the executed checks below. Z3, a declared-duration fuzz campaign, the OCI image, benchmark evidence, and a complete release qualification bundle were not executed in this environment. |
| OS-004 / OS-005 — Embedded-key trust and overbroad `valid` result | **Core semantics clarified; Enterprise trust remains absent** | Verification now reports `verification_scope: integrity_and_binding_only`. Tests cover unsigned internally consistent artifacts, self-signed embedded keys, and tampering. Core still has no external issuer trust store, tenant binding or revocation; `valid` must not authorize deployment. |
| OS-006 — Strategy names mistaken for semantics | **Documentation clarified; strategy-specific proofs absent** | The language spec now says the strategy registry and digest identify a profile and do not prove its algebraic laws. The certificate covers the checked user-declared relation. Strategy-specific proof obligations require a new profile. |
| OS-007 — Linear tokens mistaken for runtime enforcement | **Bounded in documentation** | The language spec states that token counting is compile-time only and does not provide runtime uniqueness, persistence, consensus or double-spend prevention. |
| OS-008 — Self-declared authority | **Bounded in documentation** | The trust model says `control_plane` is a modeling declaration, not remote attestation or provenance verification. |
| OS-009 — Offline verification does not rerun obligations | **Explicitly scoped** | The certificate guide, API docs and response schema state integrity/binding scope; verification does not rerun SMT or prove claims. |
| OS-010 — Reference HTTP service is not a tenant API | **Partially hardened; Enterprise controls absent** | Non-loopback binding requires a bearer token and API routes require it. There is no tenant model, SSO, RBAC/ABAC or control plane. Keep this adapter internal or local. |
| OS-011 — Key file is not enterprise custody | **Not an Enterprise signer** | The Core supports an optional local key file. KMS/HSM custody, issuer identity, rotation and revocation remain absent and are documented as deployment responsibilities. |
| OS-012 — Mutable build inputs | **Partially addressed** | Cargo inputs are locked and release tag/version alignment is checked. Docker base images, distribution Z3 packages and GitHub Actions are still selected by mutable tags rather than immutable digests/SHAs. |
| OS-013 — Durable evidence registry | **Not in Core scope** | The reference service is stateless and has no evidence lifecycle, WORM retention, legal hold or searchable audit registry. |
| OS-014 — Worker/PID resource model | **Core output bounded; worker fleet not production-qualified** | Z3 stdout and stderr now each have an 8 MiB cap and overflow fails closed. The reference service has request-body, compile-time, obligation and concurrency limits (default 32). Per-job process/PID, memory, CPU and worker-fleet concurrency have not been qualified. The Unix fake-process overflow regression is configured for CI; only the bounded-reader unit test ran on this Windows host. |
| OS-015 — Benchmark definitions without evidence | **Open** | A Criterion benchmark exists; this audit did not produce a retained workload/hardware qualification result or SLO. |
| OS-016 — No proof-producing solver | **Documented limitation** | The artifact records solver results and identities; it contains no independently checkable SMT proof or compiler-correctness proof. |
| Section 3.4 — Schema and API mismatches | **Open** | The contract schema permits identifiers the compiler rejects, does not precisely state the Rust `i64` range, and does not encode all transition conditionals. OpenAPI does not describe the Enterprise auth/tenant/idempotency contract. Service readiness currently shares the health handler. These must be resolved or explicitly documented before claiming schema/API conformance. |

## Enterprise v1.0 scope not implemented in this repository

The baseline assigns the following to Enterprise rather than Core. They are
not satisfied by the present repository:

- SSO/OIDC, SCIM, organizations, tenants, projects, membership and
  authorization policy.
- Immutable contract and policy registries, queued jobs, leases, retries and
  cross-service idempotency.
- Isolated worker fleet with pinned solver images, process-group cancellation,
  bounded stdout/stderr and measured per-job resource budgets.
- Durable artifact/evidence storage, atomic issuance transactions, retention,
  legal hold and audit hash-chain verification.
- Tenant-scoped issuer/key trust, KMS/HSM signing, approvals, separation of
  duties, key/certificate revocation and trust-aware verification.
- Git provider checks, Kubernetes admission, workload-image digest binding,
  portal, SIEM integration, multi-AZ availability, backup/restore and DR.

Sections 26 and 27 of the Enterprise specification require those capabilities,
acceptance scenarios AS-01 through AS-10, security review, production
qualification and a signed evidence bundle before Enterprise GA. This Core
audit does not mark those gates complete.

## Verification run for this audit

| Check | Outcome | Notes |
|---|---|---|
| `cargo test --workspace --all-features --locked` | **Passed** on pinned Rust 1.85.0 | Includes compiler, adversarial, conformance, stress, service, solver, syntax and property tests. The new bounded finite/Z3 test detects that Z3 is absent and skips the external comparison. |
| `cargo +stable clippy --workspace --all-targets --all-features --locked -- -D warnings` | **Passed** | Stable toolchain is Rust 1.98.0 in this environment. The pinned 1.85 Clippy launcher reports that its component is not applicable, despite Rust/Cargo 1.85 working. |
| `cargo +stable fmt --all --check` | **Passed** | The pinned 1.85 formatter launcher reports the same local component issue; stable formatting check succeeds. |
| `cargo +stable build --workspace --release --locked` | **Passed** | Windows release binaries were built. |
| Finite-solver E2E: valid `capacity.json` compile and offline verification | **Passed** | Result `STATICALLY_CERTIFIED`; verification returns `valid: true` with scope `integrity_and_binding_only`. |
| Finite-solver E2E: invalid counterexample compile | **Passed** | Result `INVARIANT_VIOLATED`, exit code 3, and no artifact directory. |
| Z3 interoperation and smoke test | **Not run** | `z3` is not installed or on PATH. The finite/Z3 differential test is present for environments where Z3 is installed; this run did not compare solver answers. |
| Fuzz target build/campaign | **Not run** | Cargo could not write the `libfuzzer-sys` registry cache under the restricted user Cargo directory. No fuzz duration or crash-free campaign is claimed. |
| OCI image build/smoke test | **Not run** | Docker client exists, but access to the Docker Desktop engine named pipe returned permission denied. |
| Benchmark qualification | **Not run** | No workload/hardware result was retained. |
| Full `scripts/verify.sh` | **Not run as a script** | The Windows environment lacks a usable `sh`; its available Rust checks were run natively where possible. |

## Release posture

Do not treat this result as approval to tag `0.1.1`, publish an OCI image, or
market Enterprise v1.0 as production-ready. Close the remaining Core
qualification gates first: run the Z3 differential and end-to-end suite, build
and smoke-test a digest-pinned image, execute and retain the fuzz and benchmark
results, repair the local pinned Rust tooling issue or qualify a clean pinned
toolchain, and publish the complete release evidence. Enterprise implementation
and review gates remain a separate program of work.

### Practical next steps for the open-source Core

The repository already configures GitHub Actions to install Z3 and run both
finite and Z3 end-to-end smoke tests on pull requests (`.github/workflows/ci.yml`).
The fuzz workflow runs a short campaign on pull requests and a longer scheduled
campaign (`.github/workflows/fuzz.yml`). These are configured checks, not proof
that the current working changes passed them. The audit environment has no
successful run attached to this working tree, and its local Z3/fuzz limitations
remain as recorded above.

Proceed in this order before making a broader Core claim:

1. Put the current changes on a review branch and open a pull request. Do not
   merge or release them yet.
2. Wait for the pull-request CI and fuzz jobs to finish. Require green results
   for formatting, Clippy, locked workspace tests, release build, both solver
   smoke tests, and every fuzz target. A missing or skipped Z3 check does not
   qualify Z3 support.
3. If a job fails, fix the cause on that branch and rerun the checks. Retain
   the GitHub Actions run links and fuzz artifacts with the release evidence.
4. Before a release, run `scripts/verify.sh` in a clean Linux environment with
   the pinned Rust toolchain and Z3 installed, then retain the output. Run and
   retain the scheduled fuzz campaign and benchmark results as separate gates.
5. Keep the public claim scoped to the modes and guarantees supported by that
   evidence. In particular, a passing artifact-integrity check is not an
   independent proof of compiler semantics, and the finite solver remains
   bounded-domain.

The CI workflow uses hosted Ubuntu runners, so these checks do not depend on
installing Z3 or Docker on the developer's Windows machine. Container image
build and smoke testing remain a separate distribution gate; they do not block
the finite/Z3 source-level compiler checks from running in CI.

## Document inspection note

The specification's paragraphs, headings and tables were extracted from its
DOCX package. The package contained no tracked insertions/deletions or comments.
Visual page-layout review was unavailable because LibreOffice is not installed
in this environment; the source document's layout was not used as evidence for
the code audit.
