//! Version identifiers that are part of the public compatibility contract.

/// Compiler implementation version.
pub const COMPILER_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Text language version implemented by this release.
pub const LANGUAGE_VERSION: &str = "0.1";
/// JSON contract projection schema version.
pub const CONTRACT_SCHEMA_VERSION: &str = "0.1";
/// Canonical hard-manifest schema version.
pub const MANIFEST_SCHEMA_VERSION: &str = "0.1";
/// λ-certificate schema version.
pub const CERTIFICATE_SCHEMA_VERSION: &str = "0.1";
/// Supported decision theory identifier.
pub const THEORY_ID: &str = "QF_LIA";
/// Canonicalization profile used before hashing.
pub const CANONICALIZATION_ID: &str = "RFC8785-JCS";
/// Digest algorithm used by v0.1 artifacts.
pub const DIGEST_ALGORITHM: &str = "SHA-256";

