# Architecture

## Data flow

```text
Text or JSON
  -> parse/lower
  -> names, types and units
  -> authority and hard-fragment gate
  -> assumption feasibility
  -> invariant validity
  -> transition feasibility/preservation
  -> canonical hard manifest
  -> λ-certificate
  -> optional Ed25519 signature
```

No artifact is constructed before all hard solver obligations succeed.

## Dependency direction

```text
model
  ^
  |-- syntax
  |-- check
  |    ^
  |    |-- solver
  |    |-- emit
  |         ^
  +---------|-- compiler
                 ^
                 |-- CLI
                 +-- HTTP service
```

The model crate has no I/O or cryptography. The checker has no solver dependency.
The solver interface accepts checked expressions. Artifact emission has no
network or filesystem access. CLI and service adapters own external I/O.

## Parser

The lexer records byte, line and column spans. The recursive-descent parser has
explicit precedence and produces an expression tree rather than storing opaque
formula strings. JSON expressions are routed through the same expression parser,
so the two source formats cannot acquire different expression semantics.

## Typed core

The checker resolves alias scales and lowers variables into canonical base-unit
expressions. Compile-time constants are substituted. Source spans are retained
for diagnostics but excluded from hard serialization. Multiplication is
rejected unless one operand is a dimensionless compile-time integer.

## Solver boundary

`DecisionProcedure` exposes `identity()` and `solve(query, timeout)`. The compiler
encodes validity as satisfiability of a negated obligation. The Z3 adapter emits
SMT-LIB with `QF_LIA`, declared bounds, model production and an internal timeout;
it also applies an external process deadline and kills overruns.

The finite solver enumerates referenced Boolean and explicitly bounded integer
domains. It reports `Unknown` for missing bounds, excessive domain size or state
count. It never treats bounded search as evidence about an unbounded domain.

## Artifact boundary

`HardManifest` contains only hard-relevant declarations, normalized assumptions,
certified invariants, verified canonical transition relations, linear tokens,
the strategy-registry digest and an explicit trust boundary. `CompileResult` separately carries
diagnostics, solver records and excluded claims.

## Stateless service

The service stores no contracts or artifacts, performs no remote fetches and
constructs its solver before binding the listener. Request/body and compiler
limits are independent. Status mapping is:

| Compiler status | HTTP status |
|---|---:|
| `STATICALLY_CERTIFIED` | 200 |
| `INVARIANT_VIOLATED`, `INFEASIBLE_CONTEXT` | 409 |
| malformed, unsupported, authority violation | 422 |
| `INDETERMINATE` | 503 |

The service bounds body/source size, generated solver obligations and in-process
compiler work. Work permits move into blocking tasks, so cancellation cannot
detach uncounted solver work; saturation fails fast while health probes remain
independent. The parser separately bounds token count and recursive expression
nesting.

## Extension rules

- New hard syntax requires a new language version and decision rule.
- New theories require a distinct theory id and conformance suite.
- New reconciliation strategies require versioned registry semantics; registry
  changes alter its digest.
- A commercial layer may wrap this API but cannot alter hard meanings while
  continuing to issue v0.1 certificates.
