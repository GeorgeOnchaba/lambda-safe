use crate::StrategyDefinition;
use lambda_safe_model::{
    AuthorityClass, ClaimClass, CompileStatus, Diagnostic, ExcludedClaim, Expr, TransitionKind,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Canonical value sort used by the solver interface.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CoreType {
    /// Boolean.
    Boolean,
    /// Integer expressed in a canonical base unit.
    Integer {
        /// Canonical base unit.
        base_unit: String,
        /// Scale from nominal symbol units to the base unit.
        scale: i64,
    },
}

/// Checked input symbol.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreSymbol {
    /// Stable name.
    pub name: String,
    /// Checked type.
    pub value_type: CoreType,
    /// Authority.
    pub authority: AuthorityClass,
    /// Inclusive nominal lower bound.
    pub minimum: Option<i64>,
    /// Inclusive nominal upper bound.
    pub maximum: Option<i64>,
}

/// Checked hard claim.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreClaim {
    /// Stable id.
    pub id: String,
    /// Normalized expression with constants and unit scales lowered.
    #[serde(skip_serializing)]
    pub expression: Expr,
    /// Unambiguous canonical prefix representation.
    pub canonical_expression: String,
}

/// Checked transition branch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreTransitionBranch {
    /// Branch name.
    pub name: String,
    /// Normalized present/post relation.
    #[serde(skip_serializing)]
    pub relation: Expr,
    /// Canonical relation.
    pub canonical_relation: String,
}

/// Checked transition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreTransition {
    /// Stable id.
    pub id: String,
    /// Degrade or reconcile.
    pub kind: TransitionKind,
    /// Registered strategy for reconciliation.
    pub strategy: Option<String>,
    /// Hard invariant identifiers to preserve.
    pub preserves: Vec<String>,
    /// Checked branches.
    pub branches: Vec<CoreTransitionBranch>,
}

/// Semantically checked contract ready for solver discharge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckedContract {
    /// Contract identity.
    pub contract_id: String,
    /// Contract version.
    pub contract_version: String,
    /// Language version.
    pub language_version: String,
    /// Symbols in lexical order.
    pub symbols: BTreeMap<String, CoreSymbol>,
    /// Hard assumptions.
    pub assumptions: Vec<CoreClaim>,
    /// Hard invariants.
    pub invariants: Vec<CoreClaim>,
    /// Checked transitions.
    pub transitions: Vec<CoreTransition>,
    /// Linear tokens consumed exactly once.
    pub linear_tokens: Vec<String>,
    /// Built-in strategy registry.
    pub strategies: Vec<StrategyDefinition>,
    /// Trusted and soft claims excluded from hard artifacts.
    pub excluded_claims: Vec<ExcludedClaim>,
    /// Canonical hard-only input used for deterministic hashing.
    pub hard_input: CanonicalHardInput,
}

/// Canonical input projection that deliberately excludes metadata and every
/// trusted/soft claim.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalHardInput {
    /// Language version.
    pub language_version: String,
    /// Contract identity.
    pub contract_id: String,
    /// Contract version.
    pub contract_version: String,
    /// Hard-relevant symbols.
    pub symbols: Vec<CoreSymbol>,
    /// Hard assumptions.
    pub assumptions: Vec<CoreClaim>,
    /// Hard invariants.
    pub invariants: Vec<CoreClaim>,
    /// Hard transitions.
    pub transitions: Vec<CoreTransition>,
    /// Linear tokens.
    pub linear_tokens: Vec<String>,
}

/// Static-check failure before solver invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticFailure {
    /// Fail-closed status.
    pub status: CompileStatus,
    /// Diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

/// Returns a stable string for a claim class.
#[must_use]
pub const fn claim_class_name(class: ClaimClass) -> &'static str {
    match class {
        ClaimClass::Hard => "hard",
        ClaimClass::Trusted => "trusted",
        ClaimClass::Soft => "soft",
    }
}
