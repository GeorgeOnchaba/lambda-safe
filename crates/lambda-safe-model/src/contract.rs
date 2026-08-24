use crate::{AuthorityClass, ClaimClass, Expr, Provenance, SourceSpan, ValueType};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Unit declaration. A base unit uses its own name as `base` and scale one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitDefinition {
    /// Unique unit name.
    pub name: String,
    /// Base unit name.
    pub base: String,
    /// Positive integer multiplier into the base unit.
    pub scale: i64,
    /// Source location.
    #[serde(default)]
    pub span: SourceSpan,
}

/// Typed compile-time integer constant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstantDefinition {
    /// Unique constant name.
    pub name: String,
    /// Integer value in the declared unit.
    pub value: i64,
    /// Declared unit name.
    pub unit: String,
    /// Source location.
    #[serde(default)]
    pub span: SourceSpan,
}

/// Runtime input declaration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolDefinition {
    /// Unique symbol name.
    pub name: String,
    /// Boolean or unit-bearing integer type.
    pub value_type: ValueType,
    /// Evidence/enforcement authority.
    pub authority: AuthorityClass,
    /// Optional inclusive lower bound for integer inputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i64>,
    /// Optional inclusive upper bound for integer inputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<i64>,
    /// Versioned producer provenance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
    /// Source location.
    #[serde(default)]
    pub span: SourceSpan,
}

/// Named assumption.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssumptionDefinition {
    /// Stable identifier.
    pub id: String,
    /// Hard, trusted or soft classification.
    pub class: ClaimClass,
    /// Boolean expression.
    pub expression: Expr,
    /// Source location.
    #[serde(default)]
    pub span: SourceSpan,
}

/// Named invariant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvariantDefinition {
    /// Stable identifier.
    pub id: String,
    /// Hard, trusted or soft classification.
    pub class: ClaimClass,
    /// Boolean expression.
    pub expression: Expr,
    /// Source location.
    #[serde(default)]
    pub span: SourceSpan,
}

/// State-transition category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionKind {
    /// Primary/fallback transition pair.
    Degrade,
    /// Merge transition with a registered strategy.
    Reconcile,
}

/// One relation in a transition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionBranch {
    /// Branch name (`primary`, `fallback` or `merge`).
    pub name: String,
    /// Relation over present variables and `next(variable)` references.
    pub relation: Expr,
    /// Source location.
    #[serde(default)]
    pub span: SourceSpan,
}

/// Transition whose feasibility and preservation obligations are checked.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionDefinition {
    /// Stable identifier.
    pub id: String,
    /// Degrade or reconcile.
    pub kind: TransitionKind,
    /// Required for reconcile transitions and absent for degrade transitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    /// Hard invariant identifiers each branch must preserve.
    pub preserves: Vec<String>,
    /// Relations to verify.
    pub branches: Vec<TransitionBranch>,
    /// Source location.
    #[serde(default)]
    pub span: SourceSpan,
}

/// A linearly consumed compile-time capability.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinearTokenDefinition {
    /// Token name. Declaration mints exactly one token.
    pub name: String,
    /// Source location.
    #[serde(default)]
    pub span: SourceSpan,
}

/// Token consumption event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenConsume {
    /// Referenced token.
    pub name: String,
    /// Source location.
    #[serde(default)]
    pub span: SourceSpan,
}

/// Fully parsed contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractDocument {
    /// JSON projection schema version.
    pub schema_version: String,
    /// λ-Safe language version.
    pub language_version: String,
    /// Stable contract identifier.
    pub contract_id: String,
    /// Caller-managed contract version.
    pub contract_version: String,
    /// Unit definitions.
    #[serde(default)]
    pub units: Vec<UnitDefinition>,
    /// Compile-time constants.
    #[serde(default)]
    pub constants: Vec<ConstantDefinition>,
    /// Runtime symbols.
    #[serde(default)]
    pub symbols: Vec<SymbolDefinition>,
    /// Assumptions.
    #[serde(default)]
    pub assumptions: Vec<AssumptionDefinition>,
    /// Invariants.
    #[serde(default)]
    pub invariants: Vec<InvariantDefinition>,
    /// State transitions.
    #[serde(default)]
    pub transitions: Vec<TransitionDefinition>,
    /// Minted linear tokens.
    #[serde(default)]
    pub tokens: Vec<LinearTokenDefinition>,
    /// Linear token consumption events.
    #[serde(default)]
    pub consumes: Vec<TokenConsume>,
    /// Non-hard metadata. This map is intentionally excluded from hard output.
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

/// JSON unit projection without parser-owned source locations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitProjection {
    /// Unique unit name.
    pub name: String,
    /// Base or parent unit name.
    pub base: String,
    /// Positive integer multiplier into the parent unit.
    pub scale: i64,
}

/// JSON constant projection without parser-owned source locations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstantProjection {
    /// Unique constant name.
    pub name: String,
    /// Integer value in the declared unit.
    pub value: i64,
    /// Declared unit name.
    pub unit: String,
}

/// JSON symbol projection without parser-owned source locations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SymbolProjection {
    /// Unique symbol name.
    pub name: String,
    /// Boolean or unit-bearing integer type.
    pub value_type: ValueType,
    /// Evidence/enforcement authority.
    pub authority: AuthorityClass,
    /// Optional inclusive lower bound.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i64>,
    /// Optional inclusive upper bound.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<i64>,
    /// Optional non-hard producer provenance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

/// JSON linear-token projection without parser-owned source locations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedProjection {
    /// Token name.
    pub name: String,
}

/// JSON-friendly assumption containing an expression string.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssumptionProjection {
    /// Stable identifier.
    pub id: String,
    /// Hard, trusted or soft classification.
    pub class: ClaimClass,
    /// λ-Safe expression.
    pub expression: String,
}

/// JSON-friendly invariant containing an expression string.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvariantProjection {
    /// Stable identifier.
    pub id: String,
    /// Hard, trusted or soft classification.
    pub class: ClaimClass,
    /// λ-Safe expression.
    pub expression: String,
}

/// JSON-friendly transition branch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransitionBranchProjection {
    /// Branch name.
    pub name: String,
    /// Relation over present and post-state variables.
    pub relation: String,
}

/// JSON-friendly transition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransitionProjection {
    /// Stable identifier.
    pub id: String,
    /// Transition category.
    pub kind: TransitionKind,
    /// Reconciliation strategy when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    /// Hard invariants to preserve.
    pub preserves: Vec<String>,
    /// Transition relations.
    pub branches: Vec<TransitionBranchProjection>,
}

/// Operational JSON projection accepted by the compiler.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractProjection {
    /// Projection schema version.
    pub schema_version: String,
    /// λ-Safe language version.
    pub language_version: String,
    /// Stable contract identifier.
    pub contract_id: String,
    /// Caller-managed contract version.
    pub contract_version: String,
    /// Unit definitions.
    #[serde(default)]
    pub units: Vec<UnitProjection>,
    /// Constants.
    #[serde(default)]
    pub constants: Vec<ConstantProjection>,
    /// Runtime inputs.
    #[serde(default)]
    pub symbols: Vec<SymbolProjection>,
    /// Assumptions.
    #[serde(default)]
    pub assumptions: Vec<AssumptionProjection>,
    /// Invariants.
    #[serde(default)]
    pub invariants: Vec<InvariantProjection>,
    /// Transitions.
    #[serde(default)]
    pub transitions: Vec<TransitionProjection>,
    /// Minted tokens.
    #[serde(default)]
    pub tokens: Vec<NamedProjection>,
    /// Token consumption events.
    #[serde(default)]
    pub consumes: Vec<NamedProjection>,
    /// Non-hard metadata excluded from certification.
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}
