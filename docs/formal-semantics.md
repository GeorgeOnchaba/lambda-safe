# Formal Semantics and Decision Rules

## Contract model

A parsed contract is represented as:

\[
K = (D, C, A, I, T, L, M, V)
\]

where `D` contains runtime declarations, `C` typed constants, `A` assumptions,
`I` invariants, `T` transition relations, `L` linear capabilities, `M`
non-hard metadata and `V` schema/language versions.

The ordinary typing judgment is:

\[
\Gamma_t \vdash e : \tau [u,a]
\]

where `τ` is `Bool` or `Int`, `u` is a canonical base unit or `unitless`, and
`a` is the authority requirement accumulated from referenced declarations.

## Eligibility

A claim is hard eligible exactly when:

\[
Eligible(I) \equiv Typed(I) \land UnitSafe(I) \land QF\text{-}LIA(I)
\land Authority(I) \subseteq \{ControlPlane, Constant\}
\]

Classification as `hard` is necessary but not sufficient. Eligibility and
solver discharge are separate gates.

## Non-elevation

Let `H`, `R` and `S` be hard, trusted-report and soft claim sets. The compiler
enforces:

\[
Manifest.hard \subseteq H
\]

and:

\[
(R \cup S) \cap Manifest.hard = \varnothing
\]

No transformation rule converts a trusted or soft claim into a hard claim.
Inputs used only by `R` or `S` are absent from the canonical hard input.

## Feasibility

After normalization, hard assumptions and declared symbol ranges form `Γ`.

\[
SAT(\Gamma) \Rightarrow Feasible
\]

\[
UNSAT(\Gamma) \Rightarrow INFEASIBLE\_CONTEXT
\]

`UNKNOWN`, timeout or solver failure maps to `INDETERMINATE`. An infeasible
context is rejected before claim validity so contradiction cannot certify every
claim vacuously.

## Validity

For each hard invariant `I`:

\[
UNSAT(\Gamma \land \neg I) \Rightarrow \Gamma \models I
\]

\[
SAT(\Gamma \land \neg I) \Rightarrow INVARIANT\_VIOLATED
\]

The satisfying model is returned as a counterexample when the solver exposes
one. `UNKNOWN`, timeout and failure are indeterminate.

## Transition feasibility and preservation

For branch relation `R(pre,post)` preserving hard invariants
`P = {I₁, …, Iₙ}`, define:

\[
Pre = \Gamma \land \bigwedge_{I \in P} I(pre)
\]

The branch must be feasible:

\[
SAT(Pre \land R)
\]

and each invariant must be preserved:

\[
UNSAT(Pre \land R \land \neg I_k(post))
\]

An infeasible relation is rejected instead of being treated as a vacuous
preservation proof.

## Linear context

Token declarations form linear context `Δ`. Each declared token `t` must satisfy:

\[
count_{consume}(t) = 1
\]

The v0.1 language does not expose token duplication or implicit weakening.

## Deterministic artifacts

Source spans, comments, whitespace, metadata, trusted claims and soft claims are
removed from the canonical hard projection. Ordered vectors are sorted by stable
identifiers, maps are lexical, integer values contain no floating-point
ambiguity, and serialization uses RFC 8785 JSON Canonicalization Scheme.

The certificate digest is:

\[
LC = SHA256(JCS(CertificateBody))
\]

The optional Ed25519 signature authenticates the raw 32-byte value represented
by `LC`. The signature is outside `CertificateBody`, preventing a digest cycle.

## Fail-closed theorem implemented as an invariant

The implementation enforces the structural invariant:

\[
artifacts \neq None \Rightarrow status = STATICALLY\_CERTIFIED
\]

Every other status returns `artifacts = None`. Tests cover malformed input,
authority crossing, unsupported formulas, infeasible assumptions,
counterexamples, transition defects, unknown results, resource limits and
artifact substitution.

## Soundness boundary

This implementation provides decision-procedure-backed verification of the
normalized model. It does not include a mechanized proof of parser, typechecker,
normalizer or compiler correctness. Z3 or the exact bounded reference solver,
the Rust implementation, dependencies and platform are part of the trusted
computing base.

