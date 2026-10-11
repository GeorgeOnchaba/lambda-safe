use crate::{Diagnostic, PrimitiveValue};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Top-level fail-closed compilation outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompileStatus {
    /// Every required hard gate succeeded.
    StaticallyCertified,
    /// At least one hard invariant has a counterexample.
    InvariantViolated,
    /// Hard assumptions are mutually inconsistent.
    InfeasibleContext,
    /// Source uses a construct outside the supported language or theory.
    Unsupported,
    /// Solver or resource-bounded execution could not decide safely.
    Indeterminate,
    /// Input could not be parsed or structurally validated.
    MalformedContract,
    /// A hard claim crossed the declared authority boundary.
    AuthorityViolation,
}

impl CompileStatus {
    /// Whether hard artifacts may be emitted.
    #[must_use]
    pub const fn permits_certificate(self) -> bool {
        matches!(self, Self::StaticallyCertified)
    }
}

/// Solver implementation recorded in verification evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolverIdentity {
    /// Adapter name.
    pub name: String,
    /// Solver version or deterministic implementation identifier.
    pub version: String,
    /// Supported SMT theory.
    pub theory: String,
    /// Deterministic options relevant to the result.
    pub options: BTreeMap<String, String>,
}

/// Counterexample produced for a satisfiable negated obligation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Counterexample {
    /// Parsed assignments when available.
    #[serde(default)]
    pub assignments: BTreeMap<String, PrimitiveValue>,
    /// Sanitized raw solver model when parsing is incomplete.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_model: Option<String>,
}

/// Outcome for one verification obligation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationOutcome {
    /// The queried constraints are satisfiable.
    Satisfiable,
    /// The queried constraints are unsatisfiable.
    Unsatisfiable,
    /// The solver did not produce a safe yes/no answer.
    Indeterminate,
}

/// Machine-readable record for one solver obligation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationRecord {
    /// Stable obligation identifier.
    pub obligation_id: String,
    /// Feasibility, invariant validity or transition preservation.
    pub obligation_kind: String,
    /// Solver result.
    pub outcome: VerificationOutcome,
    /// Counterexample for satisfiable violation queries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterexample: Option<Counterexample>,
}

/// Canonical declaration used by a hard artifact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestDeclaration {
    /// Name.
    pub name: String,
    /// Canonical sort (`bool` or `int`).
    pub sort: String,
    /// Canonical base unit for integers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_unit: Option<String>,
    /// Scale into the base unit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<i64>,
    /// Authority class.
    pub authority: String,
    /// Optional lower bound.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i64>,
    /// Optional upper bound.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<i64>,
}

/// Canonical claim in the hard manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestClaim {
    /// Stable identifier.
    pub id: String,
    /// Canonical expression.
    pub expression: String,
    /// Digest of the canonical claim record.
    pub digest: String,
}

/// Canonical transition-preservation result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestTransition {
    /// Transition identifier.
    pub id: String,
    /// Transition kind.
    pub kind: String,
    /// Registered strategy when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    /// Verified invariant identifiers.
    pub preserves: Vec<String>,
    /// Canonical branch-name to transition-relation mapping.
    pub relations: BTreeMap<String, String>,
    /// Digest of the canonical transition relations.
    pub relation_digest: String,
}

/// Deterministic, hard-only manifest body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HardManifest {
    /// Manifest format version.
    pub schema_version: String,
    /// λ-Safe language version.
    pub language_version: String,
    /// Compiler implementation version.
    pub compiler_version: String,
    /// Contract identity.
    pub contract_id: String,
    /// Contract version.
    pub contract_version: String,
    /// Supported logic.
    pub theory: String,
    /// Canonical hard-relevant declarations.
    pub declarations: Vec<ManifestDeclaration>,
    /// Canonical hard assumptions.
    pub assumptions: Vec<ManifestClaim>,
    /// Canonical certified invariants.
    pub invariants: Vec<ManifestClaim>,
    /// Certified transitions.
    pub transitions: Vec<ManifestTransition>,
    /// Linear token names proven to be consumed exactly once.
    pub linear_tokens: Vec<String>,
    /// Digest of the built-in strategy registry.
    pub strategy_registry_digest: String,
    /// Explicit trusted-computing and runtime boundary.
    pub trust_boundary: Vec<String>,
}

/// Unsigned certificate body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LambdaCertificateBody {
    /// Certificate format version.
    pub schema_version: String,
    /// Canonicalization profile.
    pub canonicalization: String,
    /// Digest algorithm.
    pub digest_algorithm: String,
    /// Compiler version.
    pub compiler_version: String,
    /// Solver identity.
    pub solver: SolverIdentity,
    /// Contract identity.
    pub contract_id: String,
    /// Contract version.
    pub contract_version: String,
    /// Digest of the canonical hard manifest.
    pub manifest_digest: String,
    /// Digest of the canonical hard input projection.
    pub hard_input_digest: String,
    /// Digest of canonical hard assumptions.
    pub assumptions_digest: String,
    /// Digests of accepted hard claims.
    pub hard_claim_digests: Vec<String>,
    /// Digest of the strategy registry.
    pub strategy_registry_digest: String,
}

/// Optional Ed25519 signature over the certificate digest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignatureEnvelope {
    /// Signature algorithm.
    pub algorithm: String,
    /// Base64 public key.
    pub public_key: String,
    /// Base64 signature.
    pub signature: String,
}

/// λ-certificate and optional authentication layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LambdaCertificate {
    /// Canonical certificate body.
    pub body: LambdaCertificateBody,
    /// SHA-256 digest of the canonical body.
    pub certificate_digest: String,
    /// Optional Ed25519 signature. It is not part of the digest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<SignatureEnvelope>,
}

/// Hard artifact pair returned only for static certification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompilationArtifacts {
    /// Canonical hard manifest.
    pub manifest: HardManifest,
    /// Matching λ-certificate.
    pub certificate: LambdaCertificate,
}

/// Non-hard claim reported without semantic elevation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExcludedClaim {
    /// Claim identifier.
    pub id: String,
    /// `trusted` or `soft`.
    pub class: String,
    /// Reason the claim is absent from hard artifacts.
    pub reason: String,
}

/// Complete compile response. Hard artifacts are absent for every failure
/// status and all non-hard claims remain outside the certificate digest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompileResult {
    /// Final status.
    pub status: CompileStatus,
    /// Stable diagnostics.
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
    /// Executed solver obligations.
    #[serde(default)]
    pub verification: Vec<VerificationRecord>,
    /// Trusted and soft claims excluded from hard output.
    #[serde(default)]
    pub excluded_claims: Vec<ExcludedClaim>,
    /// Hard artifacts, present only for `STATICALLY_CERTIFIED`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<CompilationArtifacts>,
}

impl CompileResult {
    /// Constructs a failure response with no hard artifacts.
    #[must_use]
    pub fn failed(status: CompileStatus, diagnostics: Vec<Diagnostic>) -> Self {
        debug_assert!(!status.permits_certificate());
        Self {
            status,
            diagnostics,
            verification: Vec::new(),
            excluded_claims: Vec::new(),
            artifacts: None,
        }
    }
}

/// Result of offline artifact verification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CertificateVerification {
    /// Whether all required format, digest, binding and supplied-signature checks passed.
    pub valid: bool,
    /// Scope of the check. This format does not contain solver proofs and does
    /// not establish that the encoded claims are true.
    pub verification_scope: CertificateVerificationScope,
    /// Individual checks.
    pub checks: BTreeMap<String, bool>,
    /// Diagnostics for failed checks.
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
}

/// What offline verification of a v0.1 certificate establishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CertificateVerificationScope {
    /// Internal format, digest and artifact bindings only; no semantic proof.
    IntegrityAndBindingOnly,
}
