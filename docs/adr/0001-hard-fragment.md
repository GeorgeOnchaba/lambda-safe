# ADR 0001: QF-LIA as the v0.1 hard fragment

- Status: Accepted
- Date: 2026-08-24

## Decision

Hard certification is restricted to authority-bounded quantifier-free linear
integer arithmetic. Boolean structure, comparisons and constant multiplication
are supported; quantifiers, floating point and variable-by-variable
multiplication are rejected.

## Rationale

The fragment is expressive for bounded capacities, quotas, routing weights and
control-plane limits while retaining a clear decidable solver boundary. A
smaller boundary is easier to explain, test and audit than an implicit
best-effort theory mix.

## Consequences

Some useful policies remain trusted or soft. New hard theories require distinct
version identifiers and evidence rather than silently broadening v0.1.
