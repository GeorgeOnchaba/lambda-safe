# MSc Thesis Concept

## Working title

**Design, Implementation and Evaluation of a λ-Safe Compiler for Decidable
Invariant Certification in Distributed Control Planes**

## Problem

Adaptive distributed control planes combine authoritative state, reported
observations and predictions. Existing policy tooling can blur those evidence
classes, accept vacuous assumptions or produce decisions that are difficult to
reproduce. The research problem is to determine whether a deliberately bounded
contract language and compiler can provide useful, explainable and reproducible
static certification without claiming safety outside its model.

## Research question

How effectively can an authority-aware, unit-safe, QF-LIA compiler detect and
explain invariant defects in distributed control-plane contracts while
remaining deterministic and practically usable?

## Objectives

1. Define a formal language, type system, authority lattice and hard decision
   boundary.
2. Implement an independent open-source Rust compiler and artifact verifier.
3. Establish non-vacuous invariant and transition obligations.
4. Evaluate correctness on adjudicated positive and adversarial corpora.
5. Measure performance, determinism and diagnostic usefulness.

## Method

Use design-science methodology: construct the language and compiler, derive a
traceable evaluation corpus, compare results with exhaustive bounded oracles
and an external SMT solver, and report both successful detections and boundary
rejections. Performance experiments should specify hardware, versions, warm-up,
sample size and confidence intervals. Diagnostic usefulness should use a
predefined rubric or participant protocol rather than anecdote.

## Primary artifact

This monorepo is the reference artifact. The normative specification and
traceability matrix are part of the research evidence; proprietary platform
features are outside the thesis intervention.

## Expected contribution

- An explicit authority-aware certification boundary.
- A non-vacuous compilation procedure for invariants and adaptive transitions.
- A reproducible hard-manifest and certificate format.
- Empirical evidence about the boundary's correctness, limits and cost.

## Limitations

The study does not prove implementation correctness, runtime enforcement,
telemetry integrity or universal distributed-system safety. Its conclusions are
bounded by the modeled fragment and evaluation corpus.
