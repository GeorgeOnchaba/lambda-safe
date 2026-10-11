# Manifest and λ-Certificate Format

## Hard manifest

The hard manifest is an RFC 8785/JCS-serializable object containing:

- schema, language and compiler versions;
- contract id and contract version;
- theory id `QF_LIA`;
- hard-relevant declarations with canonical units and authority;
- normalized hard assumptions and their digests;
- normalized certified invariants and their digests;
- verified transitions, canonical branch relations and relation digests;
- exact-once linear token names;
- strategy-registry digest;
- explicit trust-boundary statements.

It excludes source spans, formatting, metadata, trusted-report claims, soft
claims and predictive data.

## Claim digest

For claim id `id` and normalized prefix expression `expression`:

```text
claim_digest = SHA256(JCS({"id": id, "expression": expression}))
```

## Certificate body

The body records:

- certificate format and canonicalization profile;
- digest algorithm;
- compiler and solver identity;
- contract identity;
- hard-manifest digest;
- canonical hard-input digest;
- hard-assumption digest;
- ordered hard-claim digests;
- strategy-registry digest.

The certificate digest is:

```text
certificate_digest = SHA256(JCS(certificate.body))
```

## Optional signature

When configured, the compiler hex-decodes the 64-character certificate digest
to 32 bytes and signs those bytes with Ed25519. The envelope contains:

```json
{
  "algorithm": "Ed25519",
  "public_key": "base64...",
  "signature": "base64..."
}
```

The signature is outside the body and digest. Re-signing changes the envelope
but not the semantic certificate identity.

## Verification

The verification response reports `verification_scope` as
`integrity_and_binding_only`. Its `valid` field means that the supported
format, internal digests, cross-artifact bindings and any supplied signature
checks passed. An absent optional signature is permitted by the Core format.
This result does not rerun solver obligations, prove the encoded claims, or
establish who produced the artifact. When present, a signature is checked
against the public key embedded in that same artifact; Core has no external
trust store, issuer authorization or revocation check. Consumers must not use
this result alone to authorize deployment.

Offline verification recomputes and checks:

1. hard-manifest digest;
2. certificate-body digest;
3. contract identity across both artifacts;
4. compiler identity across both artifacts;
5. canonical hard-input digest reconstructed from the manifest;
6. strategy-registry digest consistency;
7. hard assumption and invariant claim digests;
8. canonical transition-relation digests;
9. supported format, theory and deterministic structure;
10. optional Ed25519 signature.

All required checks must pass. A valid digest proves integrity relative to the
supplied bytes; it does not independently rerun SMT obligations. Recompilation
with a trusted, pinned toolchain is a stronger verification path when source
and tools are available. Issuer trust and deployment compatibility require an
external verifier with tenant-scoped trust and policy data.

## Compatibility

Consumers must reject unknown major or semantic format versions. A field may not
be silently reinterpreted. Proof-assistant artifacts may later be referenced by
an outer evidence package; they cannot retroactively change the meaning of a
v0.1 certificate.
