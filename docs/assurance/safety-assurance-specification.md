# λ-Safe Core Safety Assurance Specification

**Profile:** λ-Safe Core 0.1, QF-LIA hard claims  
**Status:** Normative behavior and assurance boundary  
**Related baseline:** [Enterprise v1.0 baseline audit](enterprise-v1-baseline-audit.md)

This document defines the safety claims the public Core may make. It specifies
the intended contract and certificate boundary; it is not a machine-checked
proof that the implementation or its dependencies are correct.

## 1. Assurance objective

For a checked hard contract, Core may report `STATICALLY_CERTIFIED` only when
the declared hard context is feasible and every required hard invariant and
transition obligation is discharged by a supported decision procedure within
the compilation deadline. An unsupported construct, counterexample,
indeterminate solver result or resource limit must not produce compilation
artifacts.

The claim is relative to the submitted model, the supported language profile,
the compiler and solver implementation, and the truth of the declared inputs
and relations. It says nothing about whether a running application conforms
to that model.

## 2. Normative obligations

Let `Γ` be the conjunction of eligible hard assumptions and `I` the complete
ordered set of hard invariants. Every solver query is a satisfiability query.

1. **Context feasibility:** require `SAT(Γ)`. `UNSAT(Γ)` returns
   `INFEASIBLE_CONTEXT`; `unknown`, timeout or execution failure returns
   `INDETERMINATE`.
2. **Invariant validity:** for every `Iᵢ`, require
   `UNSAT(Γ ∧ ¬Iᵢ)`. `SAT` is a counterexample and returns
   `INVARIANT_VIOLATED`.
3. **Transition feasibility:** for every declared branch relation `R` and
   complete preserved-invariant set `P`, require satisfiability of
   `Γ ∧ P(pre) ∧ R(pre, post)`. An impossible branch cannot count as safe.
4. **Transition preservation:** for every `Iᵢ` in `P`, require
   `UNSAT(Γ ∧ P(pre) ∧ R(pre, post) ∧ ¬Iᵢ(post))`. A satisfiable query blocks
   certification.
5. **Whole-job deadline:** every solver call receives no more than the
   remaining monotonic compilation budget and its configured per-query limit.
   Exhaustion is non-certifying, including during transition feasibility.
6. **Fail-closed result:** `unknown`, timeout, malformed solver output, process
   failure, unsupported theory and resource exhaustion are non-certifying.
   If a known counterexample and an indeterminate result both occur, retain
   both records and emit no artifacts. Z3 stdout and stderr are each capped at
   8 MiB; exceeding either cap terminates the solver and fails the query.
7. **Artifact gate:** `CompilationArtifacts` may exist only for
   `STATICALLY_CERTIFIED`. Canonical hard artifacts exclude soft, trusted,
   predictive and metadata-only material.

## 3. Input and model boundaries

- The supported hard theory is quantifier-free linear integer arithmetic with
  Boolean connectives and the documented unit/constant rules.
- Unknown fields and duplicate JSON members are rejected at Core input
  boundaries.
- Authority labels describe the submitted model. They do not attest to a
  runtime data producer or establish provenance.
- A reconciliation strategy identifier and registry digest identify a
  declared profile. They do not prove strategy algebraic laws; certification
  covers the declared relation and its obligations.
- A linear-token declaration and consume count are static checks. They do not
  enforce runtime uniqueness, persistence, consensus or double-spend
  prevention.
- No input expression is executed as code and the compiler does not fetch
  referenced URLs.

## 4. Artifact and verification claims

The canonical hard manifest and certificate bind supported Core identities,
contract identity, hard-input digests and ordered claim/relation digests using
the documented canonicalization profile. Recomputing those bindings is an
integrity check, not a proof of the claims.

Offline verification reports
`verification_scope: integrity_and_binding_only`. `valid: true` means
supported structure, digest and cross-artifact binding checks passed, and any
supplied signature verifies against the public key carried by that artifact.
An optional signature does not establish producer identity, issuer
authorization, tenant, policy compatibility or revocation status. Core
verification does not rerun solver obligations or produce an independently
checkable SMT proof. A deployment decision requires an external trust-aware
verifier and policy.

## 5. Assurance evidence required before release

Every release candidate must retain evidence for:

- format, Clippy, locked workspace tests and release build on the declared
  toolchain;
- valid and invalid end-to-end contracts, including transition-bearing
  deadline cases and no-artifact failure assertions;
- finite/Z3 differential checks on a declared bounded QF-LIA corpus;
- Z3 process failure, unknown, malformed-output and timeout behavior;
- deterministic artifact replay and digest/signature tampering;
- fuzz campaigns with duration, target list and crash disposition;
- benchmarks with workload and hardware profile;
- container build and smoke tests using pinned runtime inputs;
- documented residual risks and independent review of the obligation mapping.

The [Enterprise v1.0 baseline audit](enterprise-v1-baseline-audit.md) records
which evidence was actually run for the current working tree. Missing evidence
must remain an open release gate; it must not be inferred from a successful
build or from the presence of a test or workflow definition.

## 6. Non-claims

Core 0.1 does not provide runtime enforcement, application completeness,
producer attestation, consensus, strategy-specific reconciliation proofs,
proof-assistant-checked compiler correctness, Enterprise issuer trust, tenant
isolation, evidence retention, Kubernetes admission, availability objectives
or regulatory certification. These require separate implementation and
evidence. Core's static certificate must not be described as a runtime
guarantee or automatic compliance result.
