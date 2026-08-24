# Repository Agent Rules

These rules apply to automated coding agents and human contributors working in
this repository.

## Non-negotiable semantic boundary

- Never emit `CompilationArtifacts` for a status other than
  `STATICALLY_CERTIFIED`.
- Treat solver `unknown`, timeout, protocol failure, process failure and
  resource exhaustion as non-certifying.
- Keep `SAT(Γ)` feasibility separate from `UNSAT(Γ ∧ ¬I)` validity.
- Require every transition branch to be feasible and to preserve every hard
  invariant.
- Never elevate trusted-report, predictive or soft material into the canonical
  hard input, manifest or λ-certificate.
- Any hard-language semantic change requires a new language/profile version,
  specification update and conformance cases.

## Dependency boundaries

- `lambda-safe-model` remains free of parsing, solving, cryptography, filesystem
  and network behavior.
- `lambda-safe-check` must not depend on a solver implementation.
- `lambda-safe-solver` accepts only checked core expressions.
- `lambda-safe-emit` performs no filesystem, process or network I/O.
- External I/O belongs in the CLI and service adapters.
- The open repository must not depend on proprietary NiGeNje components.

## Change requirements

- Preserve strict duplicate-member rejection for every inbound JSON boundary.
- Preserve deterministic sorting and RFC 8785/JCS hashing for hard artifacts.
- Add a negative or adversarial test for every fail-closed boundary change.
- Update the normative documentation and traceability report with semantic
  changes.
- Do not add incomplete implementations, unresolved work markers, unchecked
  arithmetic in normalization, or `unsafe` Rust.

## Required verification

Run `./scripts/verify.sh` before proposing a release. If the required Rust or Z3
tooling is unavailable, state that explicitly and do not represent structural
inspection as an executed compiler test.
