//! Normative, serialization-stable model used by every λ-Safe component.
//!
//! This crate intentionally contains no parsing, solving, filesystem or network
//! behavior. Keeping the model independent makes the certificate boundary easy
//! to review and reuse.

mod artifact;
mod contract;
mod diagnostic;
mod expr;
mod span;
mod types;
mod version;

pub use artifact::*;
pub use contract::*;
pub use diagnostic::*;
pub use expr::*;
pub use span::*;
pub use types::*;
pub use version::*;
