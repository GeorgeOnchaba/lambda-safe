# Release Procedure

This repository releases the public λ-Safe Core. It is not the λ-Safe
Enterprise v1.0 product. A successful Core release does not imply enterprise
issuer trust, tenant governance, deployment admission or runtime enforcement.

## Current automated outputs

The tag workflow validates the tag against `VERSION`, runs the repository
verification script, and publishes a source ZIP, a SHA-256 checksum text file,
an SPDX JSON SBOM and GitHub build-provenance attestation for the source ZIP.
It does not publish compiled binaries or an OCI image. The workflow does not
verify that the Git tag is signed, sign the checksum file, pin every action to
an immutable commit, or retain a qualification report. Docker base images and
the Z3 package installed from the distribution repository are not pinned by
digest. Do not claim these controls as completed release evidence.

## Core release gates

Before proposing a release:

1. Align `VERSION`, Cargo package metadata, schemas and release notes.
2. Commit `Cargo.lock`; use `--locked` for builds, tests, image builds and
   release packaging.
3. Run formatting, Clippy, workspace tests and the release build with the
   declared Rust toolchain.
4. With the qualified Z3 version installed, run the bounded finite/Z3
   differential corpus and the end-to-end smoke test using both solvers.
5. Run each fuzz target for a declared duration and retain crash-free run
   evidence and any minimized findings.
6. Run benchmarks on a documented workload and hardware profile. Publish only
   claims supported by those results.
7. Build and smoke-test the OCI image with pinned base-image and solver digests.
8. Generate and review the SBOM, vulnerability scan and build provenance.
9. Review residual security, truth, deployment and release risks. A Core
   artifact is not a proof object and does not establish runtime behavior.
10. Publish the source archive and machine-readable evidence without replacing
    an existing immutable release.

The release workflow is one piece of this process. A green source build alone
does not satisfy the qualification, container, fuzzing, provenance or
operational gates above.
