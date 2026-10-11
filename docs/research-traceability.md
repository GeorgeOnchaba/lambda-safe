# Research Traceability

The implementation is the experimental artifact for the proposed MSc study
*Design, Implementation and Evaluation of a λ-Safe Compiler for Decidable
Invariant Certification in Distributed Control Planes*.

The safety claim boundary is defined in
[the Core safety assurance specification](assurance/safety-assurance-specification.md).
The comparison against the Enterprise v1.0 implementation baseline, including
executed checks and unclosed gates, is recorded in
[the baseline audit](assurance/enterprise-v1-baseline-audit.md).

| Research concern | Normative mechanism | Implementation | Evidence |
|---|---|---|---|
| Decidable hard boundary | QF-LIA and authority eligibility | `lambda-safe-check` | fragment and authority tests |
| Non-vacuous validity | feasibility before negated validity | `lambda-safe-compiler` | infeasible/counterexample tests |
| Unit-safe control variables | canonical integer unit scales | checker normalizer | unit mismatch tests |
| Adaptive safety | feasible/preserving transition relations | compiler transition discharge | transition adversarial tests |
| Reconciliation discipline | checked user-declared relation; strategy label is not a proof of its algebraic laws | checker registry and transition obligations | registry digest, strategy tests and transition obligations |
| Linear capabilities | exact-once token context | checker token pass | linear-token negative tests |
| Reproducibility | JCS hard manifest and digests | `lambda-safe-emit` | deterministic artifact tests |
| Tool usability | library, CLI and HTTP interfaces | compiler/CLI/service crates | integration and route tests |
| Robust rejection | fail-closed status algebra | all pipeline stages | adversarial suite and fuzzing |

## Claims supported by the artifact

- A precise, implementable v0.1 hard fragment exists.
- The reference compiler implements separate static and decision-procedure
  gates for that fragment.
- The bounded reference solver can act as an exact oracle for finite cases.
- Artifacts bind the normalized hard model and decision identities.

## Claims requiring empirical work

- Comparative defect-detection rate against other policy methods.
- Performance distributions on representative industrial contracts.
- Human comprehensibility and authoring effort.
- External validity across control-plane domains.

Those questions require a preregistered corpus, baselines, measurements and
independent analysis. They are not asserted by repository completeness or unit
test counts.
