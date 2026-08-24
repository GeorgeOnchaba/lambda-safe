//! Deterministic artifact emission and offline verification.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use lambda_safe_check::{
    CanonicalHardInput, CheckedContract, CoreClaim, CoreSymbol, CoreTransition,
    CoreTransitionBranch, CoreType, StrategyDefinition, builtin_strategies,
    is_valid_identifier,
};
use lambda_safe_model::{
    AuthorityClass, CANONICALIZATION_ID, CERTIFICATE_SCHEMA_VERSION, COMPILER_VERSION,
    CertificateVerification, CompilationArtifacts, DIGEST_ALGORITHM, Diagnostic, Expr,
    HardManifest, LANGUAGE_VERSION, LambdaCertificate, LambdaCertificateBody,
    MANIFEST_SCHEMA_VERSION, ManifestClaim, ManifestDeclaration, ManifestTransition,
    SignatureEnvelope, SolverIdentity, THEORY_ID, TransitionKind, codes,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// Artifact construction or verification error.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum EmitError {
    /// Canonical JSON serialization failed.
    #[error("canonical serialization failed: {0}")]
    Canonicalization(String),
    /// A key or signature had invalid encoding or length.
    #[error("invalid cryptographic material: {0}")]
    InvalidKey(String),
}

/// Builds the deterministic hard manifest and matching λ-certificate.
pub fn build_artifacts(
    checked: &CheckedContract,
    solver: SolverIdentity,
    signing_key: Option<&SigningKey>,
) -> Result<CompilationArtifacts, EmitError> {
    let strategy_registry_digest = digest_value(&checked.strategies)?;

    let declarations = checked
        .symbols
        .values()
        .map(|symbol| {
            let (sort, base_unit, scale) = match &symbol.value_type {
                CoreType::Boolean => ("bool".to_owned(), None, None),
                CoreType::Integer { base_unit, scale } => {
                    ("int".to_owned(), Some(base_unit.clone()), Some(*scale))
                }
            };
            ManifestDeclaration {
                name: symbol.name.clone(),
                sort,
                base_unit,
                scale,
                authority: authority_name(symbol.authority).to_owned(),
                minimum: symbol.minimum,
                maximum: symbol.maximum,
            }
        })
        .collect::<Vec<_>>();

    let assumptions = checked
        .assumptions
        .iter()
        .map(|claim| manifest_claim(&claim.id, &claim.canonical_expression))
        .collect::<Result<Vec<_>, EmitError>>()?;
    let invariants = checked
        .invariants
        .iter()
        .map(|claim| manifest_claim(&claim.id, &claim.canonical_expression))
        .collect::<Result<Vec<_>, EmitError>>()?;
    let transitions = checked
        .transitions
        .iter()
        .map(|transition| {
            let relations = transition
                .branches
                .iter()
                .map(|branch| (branch.name.clone(), branch.canonical_relation.clone()))
                .collect::<BTreeMap<_, _>>();
            Ok(ManifestTransition {
                id: transition.id.clone(),
                kind: match transition.kind {
                    lambda_safe_model::TransitionKind::Degrade => "degrade",
                    lambda_safe_model::TransitionKind::Reconcile => "reconcile",
                }
                .to_owned(),
                strategy: transition.strategy.clone(),
                preserves: transition.preserves.clone(),
                relation_digest: digest_value(&relations)?,
                relations,
            })
        })
        .collect::<Result<Vec<_>, EmitError>>()?;

    let manifest = HardManifest {
        schema_version: MANIFEST_SCHEMA_VERSION.to_owned(),
        language_version: checked.language_version.clone(),
        compiler_version: COMPILER_VERSION.to_owned(),
        contract_id: checked.contract_id.clone(),
        contract_version: checked.contract_version.clone(),
        theory: THEORY_ID.to_owned(),
        declarations,
        assumptions,
        invariants,
        transitions,
        linear_tokens: checked.linear_tokens.clone(),
        strategy_registry_digest: strategy_registry_digest.clone(),
        trust_boundary: trust_boundary(),
    };

    let manifest_digest = digest_value(&manifest)?;
    let hard_input_digest = digest_value(&checked.hard_input)?;
    let assumptions_digest = digest_value(&manifest.assumptions)?;
    let hard_claim_digests = manifest
        .invariants
        .iter()
        .map(|claim| claim.digest.clone())
        .collect();
    let body = LambdaCertificateBody {
        schema_version: CERTIFICATE_SCHEMA_VERSION.to_owned(),
        canonicalization: CANONICALIZATION_ID.to_owned(),
        digest_algorithm: DIGEST_ALGORITHM.to_owned(),
        compiler_version: COMPILER_VERSION.to_owned(),
        solver,
        contract_id: checked.contract_id.clone(),
        contract_version: checked.contract_version.clone(),
        manifest_digest,
        hard_input_digest,
        assumptions_digest,
        hard_claim_digests,
        strategy_registry_digest,
    };
    let certificate_digest = digest_value(&body)?;
    let signature = signing_key
        .map(|key| sign_digest(&certificate_digest, key))
        .transpose()?;
    let certificate = LambdaCertificate {
        body,
        certificate_digest,
        signature,
    };
    Ok(CompilationArtifacts {
        manifest,
        certificate,
    })
}

/// Verifies canonical digests, cross-artifact identity, claim digests and an
/// optional Ed25519 signature.
pub fn verify_artifacts(
    manifest: &HardManifest,
    certificate: &LambdaCertificate,
) -> CertificateVerification {
    let mut checks = BTreeMap::new();
    let mut diagnostics = Vec::new();

    let _ = checks.insert(
        "format_profile".to_owned(),
        manifest.schema_version == MANIFEST_SCHEMA_VERSION
            && manifest.language_version == LANGUAGE_VERSION
            && manifest.theory == THEORY_ID
            && certificate.body.schema_version == CERTIFICATE_SCHEMA_VERSION
            && certificate.body.canonicalization == CANONICALIZATION_ID
            && certificate.body.digest_algorithm == DIGEST_ALGORITHM
            && !certificate.body.solver.name.is_empty()
            && !certificate.body.solver.version.is_empty()
            && matches!(
                certificate.body.solver.theory.as_str(),
                "QF_LIA" | "QF_LIA_FINITE_BOUNDS"
            ),
    );
    let _ = checks.insert(
        "manifest_structure".to_owned(),
        valid_manifest_structure(manifest),
    );
    let manifest_digest = digest_value(manifest).ok();
    let _ = checks.insert(
        "manifest_digest".to_owned(),
        manifest_digest.as_deref() == Some(&certificate.body.manifest_digest),
    );
    let body_digest = digest_value(&certificate.body).ok();
    let _ = checks.insert(
        "certificate_digest".to_owned(),
        body_digest.as_deref() == Some(&certificate.certificate_digest),
    );
    let _ = checks.insert(
        "contract_identity".to_owned(),
        manifest.contract_id == certificate.body.contract_id
            && manifest.contract_version == certificate.body.contract_version,
    );
    let reconstructed_hard_input_digest = reconstruct_hard_input(manifest)
        .and_then(|input| digest_value(&input).ok());
    let _ = checks.insert(
        "hard_input_digest".to_owned(),
        reconstructed_hard_input_digest.as_deref()
            == Some(certificate.body.hard_input_digest.as_str()),
    );
    let _ = checks.insert(
        "compiler_identity".to_owned(),
        manifest.compiler_version == certificate.body.compiler_version,
    );
    let _ = checks.insert(
        "strategy_registry_digest".to_owned(),
        manifest.strategy_registry_digest == certificate.body.strategy_registry_digest
            && digest_value(&builtin_strategies()).ok().as_deref()
                == Some(manifest.strategy_registry_digest.as_str()),
    );

    let mut claim_digests_valid = true;
    for claim in manifest.assumptions.iter().chain(&manifest.invariants) {
        let expected = manifest_claim(&claim.id, &claim.expression)
            .map(|value| value.digest)
            .ok();
        if expected.as_deref() != Some(&claim.digest) {
            claim_digests_valid = false;
        }
    }
    let manifest_claim_digests = manifest
        .invariants
        .iter()
        .map(|claim| claim.digest.clone())
        .collect::<Vec<_>>();
    claim_digests_valid &= manifest_claim_digests == certificate.body.hard_claim_digests;
    let _ = checks.insert("hard_claim_digests".to_owned(), claim_digests_valid);
    let _ = checks.insert(
        "assumptions_digest".to_owned(),
        digest_value(&manifest.assumptions).ok().as_deref()
            == Some(certificate.body.assumptions_digest.as_str()),
    );
    let _ = checks.insert(
        "transition_relation_digests".to_owned(),
        manifest.transitions.iter().all(|transition| {
            digest_value(&transition.relations).ok().as_deref()
                == Some(transition.relation_digest.as_str())
        }),
    );
    let _ = checks.insert(
        "digest_encoding".to_owned(),
        valid_digest(&certificate.certificate_digest)
            && valid_digest(&certificate.body.manifest_digest)
            && valid_digest(&certificate.body.hard_input_digest)
            && valid_digest(&certificate.body.assumptions_digest)
            && valid_digest(&certificate.body.strategy_registry_digest)
            && certificate.body.hard_claim_digests.iter().all(|value| valid_digest(value))
            && manifest.assumptions.iter().all(|claim| valid_digest(&claim.digest))
            && manifest.invariants.iter().all(|claim| valid_digest(&claim.digest))
            && manifest
                .transitions
                .iter()
                .all(|transition| valid_digest(&transition.relation_digest)),
    );

    let signature_valid = certificate.signature.as_ref().is_none_or(|signature| {
        verify_signature(&certificate.certificate_digest, signature).is_ok()
    });
    let _ = checks.insert("signature".to_owned(), signature_valid);

    for (name, passed) in &checks {
        if !passed {
            diagnostics.push(
                Diagnostic::error(
                    codes::CERTIFICATE_INVALID,
                    format!("artifact verification check `{name}` failed"),
                )
                .for_subject(name),
            );
        }
    }
    CertificateVerification {
        valid: checks.values().all(|passed| *passed),
        checks,
        diagnostics,
    }
}

fn valid_manifest_structure(manifest: &HardManifest) -> bool {
    let invariant_ids = manifest
        .invariants
        .iter()
        .map(|claim| claim.id.as_str())
        .collect::<Vec<_>>();
    let all_ids = manifest
        .assumptions
        .iter()
        .map(|claim| claim.id.as_str())
        .chain(manifest.invariants.iter().map(|claim| claim.id.as_str()))
        .chain(manifest.transitions.iter().map(|value| value.id.as_str()))
        .collect::<BTreeSet<_>>();
    let strategy_ids = builtin_strategies()
        .into_iter()
        .map(|strategy| strategy.id)
        .collect::<BTreeSet<_>>();
    is_valid_identifier(&manifest.contract_id)
        && !manifest.contract_version.is_empty()
        && manifest.contract_version.len() <= 128
        && manifest
            .contract_version
            .chars()
            .all(|character| !character.is_control())
        && !manifest.compiler_version.is_empty()
        && !manifest.invariants.is_empty()
        && !manifest.trust_boundary.is_empty()
        && all_ids.len()
            == manifest.assumptions.len()
                + manifest.invariants.len()
                + manifest.transitions.len()
        && strictly_sorted_by(&manifest.declarations, |value| value.name.as_str())
        && strictly_sorted_by(&manifest.assumptions, |value| value.id.as_str())
        && strictly_sorted_by(&manifest.invariants, |value| value.id.as_str())
        && strictly_sorted_by(&manifest.transitions, |value| value.id.as_str())
        && strictly_sorted_strings(&manifest.linear_tokens)
        && manifest.declarations.iter().all(|declaration| {
            is_valid_identifier(&declaration.name)
                && matches!(declaration.sort.as_str(), "bool" | "int")
                && declaration.authority == "control_plane"
                && !matches!((declaration.minimum, declaration.maximum), (Some(min), Some(max)) if min > max)
                && match declaration.sort.as_str() {
                    "bool" => declaration.base_unit.is_none()
                        && declaration.scale.is_none()
                        && declaration.minimum.is_none()
                        && declaration.maximum.is_none(),
                    "int" => declaration.base_unit.as_ref().is_some_and(|value| {
                        value == "unitless" || is_valid_identifier(value)
                    })
                        && declaration.scale.is_some_and(|value| value > 0),
                    _ => false,
                }
        })
        && manifest
            .assumptions
            .iter()
            .chain(&manifest.invariants)
            .all(|claim| is_valid_identifier(&claim.id) && !claim.expression.is_empty())
        && manifest
            .linear_tokens
            .iter()
            .all(|token| is_valid_identifier(token))
        && manifest.transitions.iter().all(|transition| {
            is_valid_identifier(&transition.id)
                && transition
                    .relations
                    .values()
                    .all(|relation| !relation.is_empty())
                && !transition.preserves.is_empty()
                && strictly_sorted_strings(&transition.preserves)
                && transition.preserves.iter().map(String::as_str).eq(invariant_ids.iter().copied())
                && match transition.kind.as_str() {
                    "degrade" => transition.strategy.is_none()
                        && transition.relations.len() == 2
                        && transition.relations.contains_key("primary")
                        && transition.relations.contains_key("fallback"),
                    "reconcile" => transition
                        .strategy
                        .as_ref()
                        .is_some_and(|value| strategy_ids.contains(value))
                        && transition.relations.len() == 1
                        && transition.relations.contains_key("merge"),
                    _ => false,
                }
        })
}

fn reconstruct_hard_input(manifest: &HardManifest) -> Option<CanonicalHardInput> {
    let symbols = manifest
        .declarations
        .iter()
        .map(|declaration| {
            let value_type = match declaration.sort.as_str() {
                "bool" => (declaration.base_unit.is_none()
                    && declaration.scale.is_none()
                    && declaration.minimum.is_none()
                    && declaration.maximum.is_none())
                .then_some(CoreType::Boolean)?,
                "int" => CoreType::Integer {
                    base_unit: declaration.base_unit.clone()?,
                    scale: declaration.scale?,
                },
                _ => return None,
            };
            (declaration.authority == "control_plane").then(|| CoreSymbol {
                name: declaration.name.clone(),
                value_type,
                authority: AuthorityClass::ControlPlane,
                minimum: declaration.minimum,
                maximum: declaration.maximum,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let assumptions = manifest
        .assumptions
        .iter()
        .map(core_claim_from_manifest)
        .collect();
    let invariants = manifest
        .invariants
        .iter()
        .map(core_claim_from_manifest)
        .collect();
    let transitions = manifest
        .transitions
        .iter()
        .map(|transition| {
            let (kind, branch_names): (TransitionKind, &[&str]) = match transition.kind.as_str() {
                "degrade" => (TransitionKind::Degrade, &["primary", "fallback"]),
                "reconcile" => (TransitionKind::Reconcile, &["merge"]),
                _ => return None,
            };
            let branches = branch_names
                .iter()
                .map(|name| {
                    Some(CoreTransitionBranch {
                        name: (*name).to_owned(),
                        relation: Expr::boolean(false),
                        canonical_relation: transition.relations.get(*name)?.clone(),
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            Some(CoreTransition {
                id: transition.id.clone(),
                kind,
                strategy: transition.strategy.clone(),
                preserves: transition.preserves.clone(),
                branches,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(CanonicalHardInput {
        language_version: manifest.language_version.clone(),
        contract_id: manifest.contract_id.clone(),
        contract_version: manifest.contract_version.clone(),
        symbols,
        assumptions,
        invariants,
        transitions,
        linear_tokens: manifest.linear_tokens.clone(),
    })
}

fn core_claim_from_manifest(claim: &ManifestClaim) -> CoreClaim {
    CoreClaim {
        id: claim.id.clone(),
        expression: Expr::boolean(false),
        canonical_expression: claim.expression.clone(),
    }
}

fn strictly_sorted_by<T, F>(values: &[T], key: F) -> bool
where
    F: for<'a> Fn(&'a T) -> &'a str,
{
    values.windows(2).all(|pair| key(&pair[0]) < key(&pair[1]))
}

fn strictly_sorted_strings(values: &[String]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Parses a 32-byte Ed25519 secret key encoded as 64 hexadecimal characters.
pub fn signing_key_from_hex(value: &str) -> Result<SigningKey, EmitError> {
    let bytes = hex::decode(value.trim()).map_err(|error| EmitError::InvalidKey(error.to_string()))?;
    let array: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        EmitError::InvalidKey(format!("expected 32 secret-key bytes, found {}", bytes.len()))
    })?;
    Ok(SigningKey::from_bytes(&array))
}

/// Canonical RFC 8785/JCS bytes.
pub fn canonical_bytes<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, EmitError> {
    serde_jcs::to_vec(value).map_err(|error| EmitError::Canonicalization(error.to_string()))
}

/// SHA-256 over canonical bytes.
pub fn digest_value<T: Serialize + ?Sized>(value: &T) -> Result<String, EmitError> {
    let canonical = canonical_bytes(value)?;
    Ok(hex::encode(Sha256::digest(canonical)))
}

/// Digest of the current strategy registry, exposed for integrations and tests.
pub fn strategy_registry_digest(
    strategies: &[StrategyDefinition],
) -> Result<String, EmitError> {
    digest_value(strategies)
}

fn manifest_claim(id: &str, expression: &str) -> Result<ManifestClaim, EmitError> {
    #[derive(Serialize)]
    struct ClaimMaterial<'a> {
        id: &'a str,
        expression: &'a str,
    }
    let digest = digest_value(&ClaimMaterial { id, expression })?;
    Ok(ManifestClaim {
        id: id.to_owned(),
        expression: expression.to_owned(),
        digest,
    })
}

fn sign_digest(digest_hex: &str, signing_key: &SigningKey) -> Result<SignatureEnvelope, EmitError> {
    let digest = hex::decode(digest_hex).map_err(|error| EmitError::InvalidKey(error.to_string()))?;
    let signature: Signature = signing_key.sign(&digest);
    Ok(SignatureEnvelope {
        algorithm: "Ed25519".to_owned(),
        public_key: STANDARD.encode(signing_key.verifying_key().to_bytes()),
        signature: STANDARD.encode(signature.to_bytes()),
    })
}

fn verify_signature(digest_hex: &str, envelope: &SignatureEnvelope) -> Result<(), EmitError> {
    if envelope.algorithm != "Ed25519" {
        return Err(EmitError::InvalidKey(format!(
            "unsupported signature algorithm `{}`",
            envelope.algorithm
        )));
    }
    let digest = hex::decode(digest_hex).map_err(|error| EmitError::InvalidKey(error.to_string()))?;
    let public_key = STANDARD
        .decode(&envelope.public_key)
        .map_err(|error| EmitError::InvalidKey(error.to_string()))?;
    let signature = STANDARD
        .decode(&envelope.signature)
        .map_err(|error| EmitError::InvalidKey(error.to_string()))?;
    let public_key: [u8; 32] = public_key.try_into().map_err(|bytes: Vec<u8>| {
        EmitError::InvalidKey(format!("expected 32 public-key bytes, found {}", bytes.len()))
    })?;
    let signature: [u8; 64] = signature.try_into().map_err(|bytes: Vec<u8>| {
        EmitError::InvalidKey(format!("expected 64 signature bytes, found {}", bytes.len()))
    })?;
    let verifying_key = VerifyingKey::from_bytes(&public_key)
        .map_err(|error| EmitError::InvalidKey(error.to_string()))?;
    let signature = Signature::from_bytes(&signature);
    verifying_key
        .verify(&digest, &signature)
        .map_err(|error| EmitError::InvalidKey(error.to_string()))
}

fn authority_name(authority: lambda_safe_model::AuthorityClass) -> &'static str {
    use lambda_safe_model::AuthorityClass::{
        Constant, ControlPlane, Predictive, TrustedReport, Unsupported,
    };
    match authority {
        ControlPlane => "control_plane",
        Constant => "constant",
        TrustedReport => "trusted_report",
        Predictive => "predictive",
        Unsupported => "unsupported",
    }
}

fn trust_boundary() -> Vec<String> {
    vec![
        "Certification is relative to the canonical hard assumptions and QF-LIA model.".to_owned(),
        "The selected SMT solver and compiler implementation are in the trusted computing base.".to_owned(),
        "The certificate does not establish application facts, telemetry truth or future runtime behavior.".to_owned(),
        "Runtime components must enforce the certified manifest and conform to declared transition relations.".to_owned(),
        "Trusted-report, predictive and soft claims are excluded from this hard artifact.".to_owned(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_safe_check::{CanonicalHardInput, CheckedContract};

    #[test]
    fn canonical_map_order_is_stable() {
        let left = BTreeMap::from([("b", 2), ("a", 1)]);
        let right = BTreeMap::from([("a", 1), ("b", 2)]);
        assert_eq!(digest_value(&left), digest_value(&right));
    }

    #[test]
    fn signs_and_verifies_digest() {
        let key = SigningKey::from_bytes(&[7_u8; 32]);
        let signature = sign_digest(&"00".repeat(32), &key).expect("signature should be created");
        assert!(verify_signature(&"00".repeat(32), &signature).is_ok());
    }

    #[test]
    fn hard_input_type_is_serializable() {
        let input = CanonicalHardInput {
            language_version: "0.1".to_owned(),
            contract_id: "x".to_owned(),
            contract_version: "1".to_owned(),
            symbols: Vec::new(),
            assumptions: Vec::new(),
            invariants: Vec::new(),
            transitions: Vec::new(),
            linear_tokens: Vec::new(),
        };
        assert!(canonical_bytes(&input).is_ok());
        let _phantom: Option<CheckedContract> = None;
    }
}
