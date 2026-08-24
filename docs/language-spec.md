# λ-Safe Language Specification 0.1

This document is normative for the v0.1 textual language. The JSON projection
in `schemas/contract.schema.json` maps to the same parsed contract model.

JSON object member names must be unique at every nesting level. Duplicate or
unknown members are rejected before semantic analysis.

## Lexical rules

- Source is UTF-8.
- Keywords and identifiers are ASCII.
- Identifiers match `[A-Za-z_][A-Za-z0-9_]{0,127}`.
- Strings use JSON escaping.
- Integer literal magnitude must fit a non-negative `i64`; unary `-` supplies
  negative values.
- Language keywords and the internal `__next` suffix are reserved.
- `#` and `//` introduce line comments.
- Whitespace and comments are semantically irrelevant.

## Grammar

```ebnf
contract        = "lambda_safe", string, ";",
                  "contract", identifier, "version", string, "{",
                  { declaration }, "}" ;

declaration     = unit_decl | const_decl | input_decl | assumption_decl
                | invariant_decl | degrade_decl | reconcile_decl
                | token_decl | consume_decl | metadata_decl ;

unit_decl       = "unit", identifier,
                  [ "=", positive_integer, identifier ], ";" ;
const_decl      = "const", identifier, ":", integer_type,
                  "=", signed_integer, ";" ;
input_decl      = "input", identifier, ":", type,
                  "authority", authority,
                  [ "range", "[", signed_integer, ",", signed_integer, "]" ], ";" ;
type            = "bool" | integer_type ;
integer_type    = "int", "<", identifier, ">" ;
authority       = "control_plane" | "trusted_report" | "predictive"
                | "unsupported" ;

assumption_decl = "assume", identifier, claim_class, ":", expression, ";" ;
invariant_decl  = "invariant", identifier, claim_class, ":", expression, ";" ;
claim_class     = "hard" | "trusted" | "soft" ;

degrade_decl    = "degrade", identifier, "preserves", id_list, "{",
                  "primary", ":", expression, ";",
                  "fallback", ":", expression, ";", "}" ;
reconcile_decl  = "reconcile", identifier, "using", identifier,
                  "preserves", id_list, "{",
                  "merge", ":", expression, ";", "}" ;
id_list         = "[", [ identifier, { ",", identifier } ], "]" ;

token_decl      = "token", identifier, ";" ;
consume_decl    = "consume", identifier, ";" ;
metadata_decl   = "metadata", identifier, "=", string, ";" ;

expression      = implication ;
implication     = disjunction, [ "=>", implication ] ;
disjunction     = conjunction, { "||", conjunction } ;
conjunction     = equality, { "&&", equality } ;
equality        = comparison, { ( "==" | "!=" ), comparison } ;
comparison      = sum, { ( "<" | "<=" | ">" | ">=" ), sum } ;
sum             = product, { ( "+" | "-" ), product } ;
product         = unary, { "*", unary } ;
unary           = ( "!" | "-" ), unary | primary ;
primary         = integer | "true" | "false" | identifier
                | "next", "(", identifier, ")"
                | "(", expression, ")" ;
```

## Units

`unit cpu_milli;` declares a base unit with scale one.

`unit cpu_core = 1000 cpu_milli;` declares an alias. An occurrence of a
`cpu_core` symbol is normalized to `1000 × symbol` in `cpu_milli`. Alias chains
are resolved transitively with checked integer scale multiplication. Cycles,
missing base units, non-positive scales and configured scale-limit violations
are rejected.

`unitless` is a built-in unit name for dimensionless compile-time constants and
cannot be redeclared.

Addition, subtraction, equality and ordering require the same canonical base
unit. Literal zero may be compared with or added to any integer unit. Other
dimensionless literals must be introduced as typed constants before comparison
with a unit-bearing value.

## Authority

| Class | Hard eligible | Meaning |
|---|---:|---|
| `control_plane` | Yes | State covered by the declared control-plane model |
| `constant` | Yes | Implicit authority of a `const` declaration |
| `trusted_report` | No | External report recorded without independent certification |
| `predictive` | No | Estimate or model output |
| `unsupported` | No | Outside the v0.1 evidence/enforcement boundary |

A hard assumption or invariant that references an ineligible operand fails with
`LSC-A001`. Reclassifying a claim as trusted or soft records it outside the hard
context and certificate.

## Hard fragment

The v0.1 hard fragment is QF-LIA with Boolean connectives. Multiplication is
accepted only when at least one operand is a dimensionless compile-time integer.
The grammar deliberately provides no quantifier, division, floating-point,
function-call, temporal, probabilistic or opaque-oracle form.

## Assumptions and invariants

Hard assumptions form context `Γ`. Trusted and soft assumptions are never added
to `Γ`. The compiler first decides `SAT(Γ)` and rejects an unsatisfiable context.
For each hard invariant `I`, the compiler then decides `SAT(Γ ∧ ¬I)`. Only
`UNSAT` certifies the invariant.

## State transitions

`next(x)` refers to the post-state value of runtime input `x`. It is legal only
inside transition relations. Constants have no post-state value.

A degradation declaration has exactly two ordered relations named `primary`
and `fallback`. A reconciliation declaration has one `merge` relation and names
a built-in versioned strategy.

Every transition must list every hard invariant in the contract. Omitting an
invariant is rejected rather than interpreted as permission to violate it. For
every branch `R` and hard invariant `I`, the compiler checks:

1. `SAT(Γ ∧ I(pre) ∧ R(pre, post))` to prevent vacuous transition claims.
2. `UNSAT(Γ ∧ I(pre) ∧ R(pre, post) ∧ ¬I(post))` for preservation.

For multiple preserved invariants, all preconditions are included in the branch
feasibility context and each postcondition is discharged separately.

The certificate covers the declared relation. It does not prove that a runtime
implementation conforms to that relation.

## Linear tokens

`token lease;` mints one compile-time linear capability. Exactly one
`consume lease;` must occur in the contract. Missing, unknown or duplicate
consumption fails with `LSC-L001`.

## Metadata

Metadata is informational. It is excluded from the canonical hard input,
manifest and certificate digest. Metadata cannot affect typing, authority,
solver queries or artifact meaning.

## Versioning

`lambda_safe "0.1";` selects the language. Unsupported versions fail closed.
Any future semantic change requires a new language version and conformance
corpus; it cannot silently reinterpret a v0.1 certificate.
