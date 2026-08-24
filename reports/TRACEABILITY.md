# Requirements Traceability

| Requirement | Normative definition | Implementation | Evidence source |
|---|---|---|---|
| Two equivalent source projections | `docs/language-spec.md` | `lambda-safe-syntax` parser and JSON lowering | parser/JSON unit tests, text and JSON examples |
| Ambiguous JSON fails closed | language and security specifications | recursive duplicate-key visitor plus `deny_unknown_fields` projections | duplicate/unknown JSON tests |
| Stable source diagnostics | `docs/diagnostics.md` | model diagnostics and all compiler stages | negative examples and adversarial tests |
| Unit-safe integers | language and formal semantics | unit resolver, checked scaling and typed lowering | checker unit tests and mismatch example |
| Authority-bounded hard claims | trust model | operand-level eligibility check | authority example and adversarial suite |
| Decidable hard fragment | ADR 0001 and formal semantics | structural QF-LIA checker | nonlinear rejection test/example |
| Non-vacuous certification | formal semantics | separate `SAT(Γ)` gate | infeasible-context tests |
| Invariant validity | formal semantics | `SAT(Γ ∧ ¬I)` obligations | positive/counterexample corpus |
| Transition feasibility | language and formal semantics | branch feasibility obligations | transition tests |
| Transition preservation | language and formal semantics | post-state invariant negation obligations | preservation adversarial case |
| Complete invariant preservation list | language specification | checker set-equality gate | omitted-invariant example |
| Reconciliation registry | architecture and ADR 0003 | deterministic built-in registry and digest | artifact verification checks |
| Linear capability discipline | language and formal semantics | exact-once declaration/consumption pass | linear-token negative case |
| Solver failure is non-certifying | ADR 0002 | `Unknown`, timeout, I/O and protocol mapping | unknown-solver adversarial test |
| Deterministic hard-only artifacts | certificate specification | canonical hard input, JCS and SHA-256 | metadata and repeatability tests |
| Artifact tamper detection | certificate and security specifications | cross-digests, structure and optional Ed25519 verification | manifest/relation tamper tests |
| Resource-bounded interfaces | architecture and API specifications | source, token, depth, node, obligation, concurrency and solver limits | limit tests and deployment config |
| Independent public use | ADR 0003 | library, CLI, service, schemas and deployment assets | smoke-test script and API tests |
| MSc artifact boundary | `research/thesis-concept.md` | complete public hard semantics | `docs/research-traceability.md` |

## Claim discipline

This matrix maps requirements to implementation and available evidence source.
It does not claim that the test commands ran in the archive-generation
environment. The exact executed/not-executed distinction is maintained in the
verification report.
