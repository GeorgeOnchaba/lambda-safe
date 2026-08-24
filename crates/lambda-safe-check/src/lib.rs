//! Static semantic boundary for λ-Safe contracts.

mod checker;
mod core;
mod strategy;

pub use checker::{CheckConfig, CheckFailure, check_contract, is_valid_identifier};
pub use core::*;
pub use strategy::{StrategyDefinition, builtin_strategies};
