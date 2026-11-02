λ-Safe Core: Technical Audit, Verification Results and Known Limitations
Audit date: 12 October 2026
Baseline specification: λ-Safe Enterprise v1.0 — Implementable Technical Specification, version 1.0, dated 28 August 2026
Scope: Public gee-cpu/lambda-safe Core repository and the Core repair and qualification requirements identified in the baseline specification.
1. Executive summary
The λ-Safe Core working tree evaluated for this audit builds, passes the workspace test suite and completes a finite-solver end-to-end artifact round-trip on Windows.

The previously identified omission of the shared transition deadline from solver calls has been fixed in the evaluated working tree and covered by a regression test.

These results support continued development and research use of the public Core within its documented limits. They do not establish that λ-Safe Enterprise v1.0 is implemented, production-ready or fully qualified.

Several qualification activities were not completed in the audit environment, including Z3 interoperation, a declared-duration fuzz campaign, OCI image build and smoke testing, and benchmark qualification. The Core also has documented limits around solver scope, trust, proof semantics, runtime resource isolation and service deployment.

The audit distinguishes:
- Checks executed successfully.
- Checks implemented but not executed in this environment.
- Capabilities that are outside the public Core repository's scope.
- Enterprise-level capabilities that require separate implementation and qualification.
2. Audited source and version identity
The Enterprise specification identifies its audited source archive by SHA-256:

4c0caf33e2a465a51827e85f739e2c69bb1d13926dd7fc280d13feb745c14188

This audit evaluates the repository tree available on 12 October 2026:
- Local base commit: c7416e8f11b9fe4024b788f284cea3dbfe31df15
- Tree: 8147d0f18b5a256de9b8a2469a6c3316e82230db
- The tree matched origin/main at audit start.
- The evaluated working tree also contained uncommitted assurance changes.
The repository VERSION is 0.1.0. This is distinct from the Enterprise artifact profile version 1.0 and the Core 0.1.1 release proposed by the specification.

The results below apply to the evaluated source and working-tree changes. They should not be interpreted as qualification of a later commit or release unless the relevant checks are rerun against that exact revision.
3. Core audit findings
Baseline reference	Result in evaluated Core	Evidence and remaining limitation
OS-001 — Transition deadline omission	Fixed and regression-tested	Every compiler solver call receives the shared deadline. The transition_feasibility_receives_whole_job_deadline adversarial test exercises a transition-bearing JSON contract and checks for INDETERMINATE with no artifacts when the transition query exhausts the job time. A valid transition example also certifies with the finite solver.
OS-002 — Missing reproducible lock	Present	Cargo.lock exists. Workspace tests, release builds, Docker builds and CI commands use --locked.
OS-003 — No executed qualification	Partially addressed	Several checks were executed; Z3, a declared-duration fuzz campaign, OCI image build, benchmark evidence and a complete release qualification bundle were not completed in this environment.
OS-004 / OS-005 — Embedded-key trust and overbroad valid result	Core semantics clarified; external issuer trust not provided	Verification reports verification_scope: integrity_and_binding_only. Tests cover unsigned internally consistent artifacts, self-signed embedded keys and tampering. Core does not provide an external issuer trust store, tenant binding or revocation. A valid result must not be interpreted as deployment authorization.
OS-006 — Strategy names mistaken for semantics	Documentation clarified; strategy-specific proofs absent	The language specification states that the strategy registry and digest identify a profile but do not prove its algebraic laws. The certificate covers the checked user-declared relation. Strategy-specific proof obligations require a separate profile.
OS-007 — Linear tokens mistaken for runtime enforcement	Bounded in documentation	Token counting is compile-time only. It does not provide runtime uniqueness, persistence, consensus or double-spend prevention.
OS-008 — Self-declared authority	Bounded in documentation	control_plane is a modelling declaration, not remote attestation or provenance verification.
OS-009 — Offline verification does not rerun obligations	Explicitly scoped	The certificate guide, API documentation and response schema state that verification checks integrity and binding. It does not rerun SMT obligations or independently prove the underlying claims.
OS-010 — Reference HTTP service is not a tenant API	Partially hardened; not a tenant API	Non-loopback binding requires a bearer token and API routes require it. The service does not provide a tenant model, SSO, RBAC/ABAC or a control plane. Keep this adapter internal or local unless separately secured and qualified.
OS-011 — Key file is not enterprise custody	Not an Enterprise signer	Core supports an optional local key file. KMS/HSM custody, issuer identity, rotation and revocation are not provided by this Core implementation.
OS-012 — Mutable build inputs	Partially addressed	Cargo inputs are locked and release tag/version alignment is checked. Docker base images, distribution Z3 packages and GitHub Actions still use mutable tags rather than immutable digests or SHAs.
OS-014 — Worker/PID resource model	Core output bounded; worker fleet not qualified	Z3 stdout and stderr each have an 8 MiB cap and overflow fails closed. The reference service has request-body, compile-time, obligation and concurrency limits (default 32). Per-job process/PID, memory, CPU and worker-fleet concurrency have not been qualified. The Unix fake-process overflow regression is configured for CI; only the bounded-reader unit test ran on the Windows host.
OS-015 — Benchmark definitions without evidence	Open	A Criterion benchmark exists, but this audit did not produce a retained workload/hardware qualification result or SLO.
OS-016 — No proof-producing solver	Documented limitation	The artifact records solver results and identities. It does not contain an independently checkable SMT proof or compiler-correctness proof.
Section 3.4 — Schema and API mismatches	Open	The contract schema permits identifiers the compiler rejects, does not precisely state the Rust i64 range, and does not encode all transition conditionals. OpenAPI does not describe an Enterprise authentication, tenant or idempotency contract. Service readiness currently shares the health handler. These issues require resolution or explicit documentation before claiming schema/API conformance.


Note: OS-013, durable evidence registry, is an Enterprise deployment capability rather than a capability implemented by the public Core. The public Core reference service is stateless and does not provide evidence lifecycle management, WORM retention, legal hold or a searchable audit registry.
4. Verification executed for this audit
Check	Outcome	Notes
cargo test --workspace --all-features --locked	Passed on pinned Rust 1.85.0	Includes compiler, adversarial, conformance, stress, service, solver, syntax and property tests. The finite/Z3 differential test detects that Z3 is absent and skips the external comparison.
cargo +stable clippy --workspace --all-targets --all-features --locked -- -D warnings	Passed	Stable toolchain was Rust 1.98.0 in this environment. The pinned 1.85 Clippy launcher reported that its component was not applicable, although Rust/Cargo 1.85 worked.
cargo +stable fmt --all --check	Passed	The pinned 1.85 formatter launcher reported the same local component issue; the stable formatting check succeeded.
cargo +stable build --workspace --release --locked	Passed	Windows release binaries were built.
Finite-solver E2E: valid capacity.json compile and offline verification	Passed	Result STATICALLY_CERTIFIED; verification returned valid: true with scope integrity_and_binding_only.
Finite-solver E2E: invalid counterexample compile	Passed	Result INVARIANT_VIOLATED, exit code 3, and no artifact directory.
Z3 interoperation and smoke test	Not run	z3 was not installed or on PATH. The audit did not compare solver answers with Z3.
Fuzz target build/campaign	Not run	Cargo could not write the libfuzzer-sys registry cache under the restricted user Cargo directory. No fuzz duration or crash-free campaign is claimed.
OCI image build/smoke test	Not run	Docker client existed, but access to the Docker Desktop engine named pipe returned permission denied.
Benchmark qualification	Not run	No workload/hardware result was retained.
Full scripts/verify.sh	Not run as a script	The Windows environment lacked a usable sh; available Rust checks were run natively where possible.


5. Scope boundary: public Core and Enterprise capabilities
The public λ-Safe Core is a compiler and artifact-verification component. It should not be represented as the complete λ-Safe Enterprise service or as the complete NiGeNje governance platform.

The following capabilities are not established by the public Core audit:
- SSO/OIDC, SCIM, organisations, tenants, projects, membership and authorization policy.
- Immutable contract and policy registries, queued jobs, leases, retries and cross-service idempotency.
- An isolated worker fleet with pinned solver images, process-group cancellation, bounded output and measured per-job resource budgets.
- Durable artifact/evidence storage, atomic issuance transactions, retention, legal hold and audit hash-chain verification.
- Tenant-scoped issuer/key trust, KMS/HSM signing, approvals, separation of duties, key/certificate revocation and trust-aware verification.
- Git provider checks, Kubernetes admission, workload-image digest binding, portal, SIEM integration, multi-AZ availability, backup/restore and disaster recovery.
These are separate Enterprise or platform-level capabilities. Their implementation and qualification must be assessed against the relevant Enterprise specification and acceptance scenarios.

The Enterprise specification requires its own acceptance scenarios, security review, production qualification and signed evidence bundle before Enterprise general availability. This Core audit does not mark those requirements complete.
6. Important interpretation limits
The following distinctions are essential when interpreting Core results:
1. Artifact integrity is not semantic proof. A successful integrity and binding check does not independently prove that the compiler's semantic claims are correct.
2. A solver result is not a proof-producing certificate. The artifact records solver results and identities; it does not contain an independently checkable SMT proof or compiler-correctness proof.
3. An embedded key is not an external trust anchor. A self-signed or internally consistent artifact does not establish a trusted issuer.
4. A Core verification result is not deployment authorization. The public Core does not supply the Enterprise issuer registry, tenant authorization or deployment approval system.
5. Finite-domain solving is bounded. A successful finite-solver result must not be generalized to unbounded domains.
6. Compile-time token counting is not runtime enforcement. It does not establish uniqueness or prevent double spending in a distributed runtime.
7. A modelling declaration is not remote attestation. A declared control_plane value does not prove the identity or provenance of a remote system.
Public descriptions, examples and release notes should preserve these distinctions.
7. Recommended Core qualification work
The repository configures GitHub Actions to install Z3 and run finite and Z3 end-to-end smoke tests on pull requests. The fuzz workflow runs a short campaign on pull requests and a longer scheduled campaign.

Configured workflows are not evidence that the evaluated working-tree changes passed those workflows.

Before a broader Core release claim:
1. Put the changes on a review branch and open a pull request.
2. Require green results for formatting, Clippy, locked workspace tests, release build, finite and Z3 solver smoke tests, and every required fuzz target.
3. If a job fails, fix the cause and rerun it. Retain the CI run links and fuzz artifacts.
4. Run scripts/verify.sh in a clean Linux environment with the pinned Rust toolchain and Z3 installed.
5. Retain the verification output.
6. Run and retain the declared fuzz campaign and benchmark results as separate qualification gates.
7. Build and smoke-test the OCI image using immutable base-image digests and pinned build inputs.
8. Resolve or explicitly document the schema/API mismatches before claiming schema/API conformance.
The CI workflow uses hosted Ubuntu runners, so Z3 and source-level compiler checks do not depend on installing Z3 or Docker on a developer's Windows machine. Container image build and smoke testing remain separate distribution checks.
8. Release posture
The evaluated Core working tree is suitable for continued development and research use within the limits described in this report.

This audit is not approval to:
- Tag or publish a new Core release.
- Publish an OCI image.
- Claim that all Core qualification gates have passed.
- Market λ-Safe Enterprise v1.0 as implemented or production-ready.
- Represent Core artifact verification as an independent proof of compiler semantics.
- Represent the public Core as a tenant-aware Enterprise service or as the complete NiGeNje platform.
Complete the outstanding Core qualification checks and retain their evidence before making broader release claims.

Enterprise implementation, integration and production qualification remain separate programmes of work.
9. Audit limitations
This audit reflects the source tree, working-tree changes and execution environment identified in Section 2. It does not establish the state of later commits unless they are independently reviewed and tested.

The audit environment did not provide a runnable Z3 installation, usable Docker engine access, the required Cargo registry cache permissions for the fuzz target, or retained benchmark results. The full shell verification script was not executed as a script.

Visual page-layout inspection of the baseline DOCX specification was unavailable because LibreOffice was not installed. The specification's extracted paragraphs, headings and tables were reviewed; page layout was not used as evidence for the code audit.
Audit conclusion: The evaluated λ-Safe Core has working compiler and artifact-verification functionality with documented limits and several executed checks. Important qualification activities remain incomplete. The public Core should be described according to the specific behaviours and verification evidence demonstrated here, not as a production-qualified Enterprise implementation.
