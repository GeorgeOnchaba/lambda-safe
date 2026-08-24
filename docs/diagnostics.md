# Diagnostic Catalogue

Codes are part of the v0.1 public contract. Messages may improve without
changing code meaning.

| Code | Category | Meaning |
|---|---|---|
| `LSC-P001` | Parse | Invalid text/JSON source or identifier |
| `LSC-P002` | Version | Unsupported schema or language version |
| `LSC-N001` | Names | Duplicate identifier |
| `LSC-N002` | Names | Unknown referenced identifier |
| `LSC-T001` | Types | Sort mismatch or invalid range use |
| `LSC-U001` | Units | Missing, cyclic, incompatible or overflowing unit |
| `LSC-A001` | Authority | Non-hard authority used by a hard expression |
| `LSC-F001` | Fragment | Formula or contract lies outside the hard fragment |
| `LSC-V001` | Verification | Assumption context is infeasible |
| `LSC-V002` | Verification | Invariant has a counterexample |
| `LSC-S001` | Solver | Timeout, unknown, protocol or process failure |
| `LSC-D001` | Transition | Degradation branch malformed, infeasible or non-preserving |
| `LSC-R001` | Transition | Reconcile strategy/relation invalid or non-preserving |
| `LSC-L001` | Linear | Token missing, unknown or consumed other than once |
| `LSC-C001` | Artifact | Canonicalization/digest/certificate construction failure |
| `LSC-C002` | Artifact | Manifest, certificate, claim digest or signature invalid |
| `LSC-X001` | Limits | Source, declaration, expression or configured resource limit |

## Outcome precedence

- A malformed source fails before static analysis.
- Static version/fragment failures return `UNSUPPORTED`.
- Authority crossing returns `AUTHORITY_VIOLATION`.
- An unsatisfiable hard context returns `INFEASIBLE_CONTEXT` before validity.
- A known invariant or transition violation takes precedence over a separate
  indeterminate obligation because the contract is already disproved.
- Only a run with no blocking diagnostic returns `STATICALLY_CERTIFIED`.

