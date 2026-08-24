# Contributing

Thank you for improving the NiGeNje λ-Safe Compiler. Changes to a
soundness-sensitive compiler require a higher evidence bar than ordinary
application changes.

## Development workflow

1. Open an issue describing the behavior, security boundary and compatibility
   impact.
2. Keep changes within one architectural layer where possible.
3. Add positive, negative and fail-closed tests for every semantic change.
4. Run `./scripts/verify.sh` before requesting review.
5. Update the language specification, schemas, diagnostics and traceability
   matrix when observable behavior changes.

All contributions must be licensed under Apache-2.0 and accompanied by a
Developer Certificate of Origin sign-off (`git commit -s`).

## Semantic-change rules

- A change that broadens the hard fragment requires a language-version change,
  a decision-rule specification and adversarial tests.
- A new theory or solver cannot silently reuse the `QF_LIA` theory id.
- A reconciliation strategy must have deterministic, versioned semantics and
  changes the strategy-registry digest.
- Trusted or soft evidence may never be elevated into the hard manifest.
- `unknown`, timeout, unsupported input and internal failure remain
  non-certifying outcomes.
- Compatibility aliases may preserve syntax but cannot weaken eligibility.

## Review requirements

At least two maintainers should review changes to parsing, normalization,
authority, solver encoding, artifact canonicalization or signature handling.
One reviewer should reason independently from the implementation and inspect
the generated obligations or artifact bytes.

## Style

- Use stable Rust and keep `unsafe` forbidden.
- Prefer explicit domain types over strings and booleans.
- Keep solver I/O behind `DecisionProcedure`.
- Return structured diagnostics instead of panicking on user input.
- Avoid nondeterministic iteration in anything included in an artifact.

## Reporting security defects

Do not open public issues for suspected certificate forgery, eligibility bypass,
solver-command injection or denial-of-service vulnerabilities. Follow
[SECURITY.md](SECURITY.md).
