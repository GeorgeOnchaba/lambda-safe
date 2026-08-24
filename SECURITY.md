# Security Policy

## Supported versions

Security fixes are provided for the latest tagged minor release. Version 0.1 is
an early research release and its API may evolve, but disclosed certificate and
soundness defects will be handled as security issues.

## Reporting

Report vulnerabilities privately to `security@nigenje.com`. Include the
affected version, minimal reproducer, expected/observed result and whether a
hard certificate was incorrectly emitted. Do not include real production
contracts, signing keys or proprietary data.

The project will acknowledge a report, establish severity and coordinate a
release before public disclosure. This document states a process, not a
guaranteed response-time service level.

## Security boundary

The compiler verifies the submitted mathematical model. It does not establish
the truth of runtime inputs, secure a deployment, enforce a transition or prove
the Rust implementation correct. The parser, checker, normalizer, solver,
canonicalizer, cryptographic dependencies and execution platform are within the
trusted computing base.

## Deployment guidance

- Run the service as a non-root user with a read-only filesystem and no outbound
  network access.
- Set request and source limits below infrastructure limits.
- Apply CPU, memory and process limits; SMT solving is adversarial work.
- Pin container images by digest and retain an SBOM and provenance.
- Keep signing keys outside images and source control. Prefer a KMS or HSM.
- Do not expose the service directly to untrusted networks without
  authentication, rate limiting and audit controls in a gateway.
- Verify every manifest/certificate pair at its consumption boundary.

## Threats specifically tested

The adversarial suite covers malformed input, unsupported theories, authority
crossing, nonlinear expressions, unit confusion, inconsistent assumptions,
counterexamples, impossible transitions, missing token consumption, duplicate
JSON members, unknown solver results and artifact substitution. Fuzz targets
cover both source projections. See [Security model](docs/security.md).
