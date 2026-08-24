# Verification Record

Date: 2026-08-24

Artifact: NiGeNje λ-Safe Compiler open-source `0.1.0` source candidate

## Executed checks

The following checks were executed against the final source tree during archive
generation:

| Check | Result |
|---|---|
| Parse all 14 TOML documents | Pass |
| Parse all 6 JSON documents with duplicate-member detection | Pass |
| Parse all 11 YAML documents/workflows | Pass |
| Resolve internal JSON Schema and OpenAPI references | Pass |
| Validate all 8 workspace-member manifests and local path dependencies | Pass |
| Validate the 8-crate Rust module/dependency graph is complete and acyclic | Pass |
| Validate release, language, schema and deployment version alignment | Pass |
| Resolve every Rust `include_str!` target | Pass |
| Resolve local Markdown links | Pass |
| Validate shell syntax and executable bits for release scripts | Pass |
| Check GitHub workflow top-level shape | Pass |
| Scan Rust delimiter/comment/string structure across 32 files | Pass |
| Scan for unresolved TODO/FIXME/HACK and placeholder macros | Pass |
| Scan production and test Rust for `unsafe` | Pass; none present |

Archive integrity and the final SHA-256 value are checked after packaging.

## Checks not executed in this generation environment

This environment did not contain `cargo`, `rustc`, `rustfmt`, Z3 or Docker.
Consequently, the following commands are defined but were **not executed here**:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --workspace --release --locked
scripts/smoke-test.sh
Z3 interoperability tests
OCI image build and runtime probes
```

No claim that these commands passed is made. `scripts/verify.sh` and the CI
workflows perform the complete executable verification in an environment with
the pinned Rust 1.85 toolchain and Z3. The script generates `Cargo.lock` when it
is absent, then uses locked dependency resolution for lint, test and release
build stages.

## Interpretation

The archive passed source-structure, metadata, reference, policy and packaging
checks available in the generation environment. Executable compiler assurance
requires running the included verification workflow; formal or mechanized
compiler correctness is outside v0.1 and is not claimed.
