# Implementation Status

Release candidate: `0.1.0`

Language profile: λ-Safe `0.1`

Manifest profile: `0.1`

Certificate profile: `0.1`

Hard theory: `QF_LIA`

## Delivered scope

The source tree is a greenfield, self-contained open-source implementation of
the v0.1 λ-Safe compiler boundary. It contains no dependency on a proprietary
NiGeNje service.

| Capability | Delivered component |
|---|---|
| Text language and strict JSON projection | `lambda-safe-syntax` |
| Versioned contract, diagnostic and artifact model | `lambda-safe-model` |
| Name, type, unit and authority analysis | `lambda-safe-check` |
| QF-LIA eligibility and canonical lowering | `lambda-safe-check` |
| Exact bounded reference decision procedure | `lambda-safe-solver` |
| SMT-LIB and external-deadline Z3 adapter | `lambda-safe-solver` |
| Feasibility, validity and transition obligations | `lambda-safe-compiler` |
| Hard manifest, JCS digests and λ-certificate | `lambda-safe-emit` |
| Optional Ed25519 signing and offline verification | `lambda-safe-emit` |
| Embedding API | `lambda-safe-compiler` |
| CLI | `lambda-safe-cli` |
| Stateless HTTP service | `lambda-safe-service` |
| Schemas and OpenAPI | `schemas/`, `api/` |
| Curated and generated test corpus | crate tests and `examples/` |
| Property, fuzz and benchmark targets | syntax tests, `fuzz/`, compiler bench |
| CI, OCI and Kubernetes operations | `.github/`, `Dockerfile`, `deploy/` |
| Normative, security and research documentation | `docs/`, `research/` |

The implementation deliberately does not claim mechanized compiler
correctness, runtime enforcement, telemetry truth, universal distributed-system
safety, SaaS tenancy, billing or proprietary orchestration features.

## Repository scale

- 8 Cargo workspace crates.
- 32 Rust source/test/benchmark/fuzz files.
- 44 declared Rust test functions, including a generated 120-case conformance
  matrix and a 100-compilation determinism exercise.
- 5 JSON Schemas and one OpenAPI document.
- 11 valid/invalid source examples.
- 4 CI/security/fuzz/release workflows.

Counts describe source coverage; they are not a substitute for executed test
results. Execution status is recorded in `reports/VERIFICATION.md`.

## Open-source boundary

Every rule needed to parse, classify, decide, emit and verify a v0.1 hard
certificate is Apache-2.0 code in this repository. A commercial wrapper may add
operations and integrations, but it cannot change v0.1 hard eligibility,
obligations or artifact meaning while retaining the same version identifiers.
