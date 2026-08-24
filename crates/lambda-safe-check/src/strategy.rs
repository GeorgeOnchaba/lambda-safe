use serde::{Deserialize, Serialize};

/// Versioned built-in reconciliation strategy admitted by v0.1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyDefinition {
    /// Stable strategy id used by contracts.
    pub id: String,
    /// Semantics version included in the strategy-registry digest.
    pub semantics_version: String,
    /// Properties required by the compiler's structural strategy gate.
    pub properties: Vec<String>,
}

/// Returns the closed, deterministic built-in strategy registry.
///
/// The compiler proves preservation against the transition relation declared in
/// the contract. Registry membership does not, by itself, prove that a runtime
/// implementation conforms to that relation.
#[must_use]
pub fn builtin_strategies() -> Vec<StrategyDefinition> {
    let mut strategies = vec![
        strategy("escrow_balance", &["bounded_accounting", "deterministic_merge"]),
        strategy("gcounter", &["monotone", "join_semilattice"]),
        strategy("lww_register", &["deterministic_tie_break"]),
        strategy("orset", &["observed_remove", "join_semilattice"]),
        strategy("pncounter", &["componentwise_monotone", "join_semilattice"]),
        strategy("restricted_map", &["componentwise_merge", "finite_keys"]),
        strategy("restricted_product", &["componentwise_merge"]),
        strategy("token_bag", &["conservation", "deterministic_merge"]),
    ];
    strategies.sort_by(|left, right| left.id.cmp(&right.id));
    strategies
}

fn strategy(id: &str, properties: &[&str]) -> StrategyDefinition {
    StrategyDefinition {
        id: id.to_owned(),
        semantics_version: "0.1".to_owned(),
        properties: properties.iter().map(|value| (*value).to_owned()).collect(),
    }
}

