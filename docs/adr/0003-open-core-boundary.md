# ADR 0003: Complete public hard semantics

- Status: Accepted
- Date: 2026-08-24

## Decision

Parsing, hard eligibility, normalization, solver obligations, manifest emission
and certificate verification are public Apache-2.0 code. Commercial products
may add hosting and enterprise operations without changing these meanings under
the same version ids.

## Consequences

Users can independently reproduce and verify hard artifacts. Differentiation is
operational rather than a hidden soundness rule.
