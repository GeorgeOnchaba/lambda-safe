# Release Procedure

1. Confirm version alignment in `VERSION`, workspace metadata, schemas and
   changelog.
2. Generate and commit `Cargo.lock` using the pinned Rust toolchain.
3. Run `./scripts/verify.sh` in a clean checkout with Z3 installed.
4. Run a time-bounded fuzz campaign for every declared target.
5. Build release binaries on supported targets and run the smoke test.
6. Generate SBOM, checksums and build provenance.
7. Sign the annotated Git tag and release checksums.
8. Publish source archive and binaries without mutable replacement.

The ZIP distributed from this workspace is a source release candidate. A
public release should complete the clean-checkout CI workflow and attach its
machine-generated evidence.
