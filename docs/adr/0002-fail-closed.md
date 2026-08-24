# ADR 0002: Fail-closed certification

- Status: Accepted
- Date: 2026-08-24

## Decision

Only definitive successful discharge creates artifacts. Timeout, `unknown`,
resource limit, unsupported input, malformed solver output and execution failure
produce an indeterminate or rejected result with no manifest or certificate.

## Consequences

Availability cannot be traded for unsound certification. Operators may retry or
select another registered decision procedure, but must never relabel an
indeterminate result as certified.
