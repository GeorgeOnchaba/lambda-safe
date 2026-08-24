use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Value sort used by declarations and the typed core.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ValueType {
    /// Boolean value.
    Boolean,
    /// Integer carrying a named unit.
    Integer {
        /// Unit identifier declared in the contract.
        unit: String,
    },
}

impl ValueType {
    /// Constructs a boolean type.
    #[must_use]
    pub const fn boolean() -> Self {
        Self::Boolean
    }

    /// Constructs an integer type with a unit.
    #[must_use]
    pub fn integer(unit: impl Into<String>) -> Self {
        Self::Integer { unit: unit.into() }
    }
}

/// Canonical unit representation. `scale` converts a declared unit into its
/// base-unit integer representation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalUnit {
    /// Canonical base unit.
    pub base: String,
    /// Positive integer scale from declared units to the base unit.
    pub scale: i64,
}

/// Authority/evidence class attached to an input declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityClass {
    /// Value originates in the control-plane state covered by the contract.
    ControlPlane,
    /// Compile-time constant declared in the contract.
    Constant,
    /// Externally reported value whose truth the compiler cannot establish.
    TrustedReport,
    /// Estimate or model output.
    Predictive,
    /// Value outside the v0.1 authority model.
    Unsupported,
}

impl AuthorityClass {
    /// Whether the authority may participate in hard assumptions and claims.
    #[must_use]
    pub const fn hard_eligible(self) -> bool {
        matches!(self, Self::ControlPlane | Self::Constant)
    }
}

/// Provenance attached to a non-constant input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    /// Stable source identity, such as a controller or telemetry producer.
    pub source: String,
    /// Version of the producing schema or component.
    pub version: String,
    /// Optional content or attestation digest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// Additional non-hard labels. These never enter the hard certificate.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,
}

/// Classification of an assumption or invariant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimClass {
    /// Eligible for static verification when every other gate succeeds.
    Hard,
    /// Depends on a declared trusted report and is excluded from hard output.
    Trusted,
    /// Preference or predictive statement excluded from hard output.
    Soft,
}

/// Primitive value in a solver counterexample.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum PrimitiveValue {
    /// Integer value.
    Integer(i64),
    /// Boolean value.
    Boolean(bool),
}
