use crate::SourceSpan;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Unary expression operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnaryOperator {
    /// Boolean negation.
    Not,
    /// Integer negation.
    Negate,
}

/// Binary expression operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BinaryOperator {
    /// Integer addition.
    Add,
    /// Integer subtraction.
    Subtract,
    /// Linear multiplication; one operand must be an integer literal.
    Multiply,
    /// Equality.
    Equal,
    /// Inequality.
    NotEqual,
    /// Less than.
    LessThan,
    /// Less than or equal.
    LessThanOrEqual,
    /// Greater than.
    GreaterThan,
    /// Greater than or equal.
    GreaterThanOrEqual,
    /// Boolean conjunction.
    And,
    /// Boolean disjunction.
    Or,
    /// Boolean implication.
    Implies,
}

/// Expression node with a source span.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Expr {
    /// Node content.
    #[serde(flatten)]
    pub kind: ExprKind,
    /// Text source location. It is omitted from canonical hard artifacts.
    #[serde(default)]
    pub span: SourceSpan,
}

/// QF-LIA expression tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExprKind {
    /// Integer literal.
    Integer { value: i64 },
    /// Boolean literal.
    Boolean { value: bool },
    /// Present-state variable or constant.
    Variable { name: String },
    /// Post-state variable, legal only inside transition relations.
    NextVariable { name: String },
    /// Unary operator.
    Unary {
        operator: UnaryOperator,
        expression: Box<Expr>,
    },
    /// Binary operator.
    Binary {
        operator: BinaryOperator,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

impl Expr {
    /// Creates an expression with an unknown source span.
    #[must_use]
    pub const fn new(kind: ExprKind) -> Self {
        Self {
            kind,
            span: SourceSpan::unknown(),
        }
    }

    /// Creates an integer literal.
    #[must_use]
    pub const fn integer(value: i64) -> Self {
        Self::new(ExprKind::Integer { value })
    }

    /// Creates a boolean literal.
    #[must_use]
    pub const fn boolean(value: bool) -> Self {
        Self::new(ExprKind::Boolean { value })
    }

    /// Creates a present-state variable.
    #[must_use]
    pub fn variable(name: impl Into<String>) -> Self {
        Self::new(ExprKind::Variable { name: name.into() })
    }

    /// Creates a post-state variable.
    #[must_use]
    pub fn next_variable(name: impl Into<String>) -> Self {
        Self::new(ExprKind::NextVariable { name: name.into() })
    }

    /// Returns every referenced base variable in lexical order.
    #[must_use]
    pub fn referenced_variables(&self) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        self.collect_variables(&mut names);
        names
    }

    fn collect_variables(&self, names: &mut BTreeSet<String>) {
        match &self.kind {
            ExprKind::Variable { name } | ExprKind::NextVariable { name } => {
                let _ = names.insert(name.clone());
            }
            ExprKind::Unary { expression, .. } => expression.collect_variables(names),
            ExprKind::Binary { left, right, .. } => {
                left.collect_variables(names);
                right.collect_variables(names);
            }
            ExprKind::Integer { .. } | ExprKind::Boolean { .. } => {}
        }
    }

    /// Returns true when the tree contains a post-state reference.
    #[must_use]
    pub fn contains_next_variable(&self) -> bool {
        match &self.kind {
            ExprKind::NextVariable { .. } => true,
            ExprKind::Unary { expression, .. } => expression.contains_next_variable(),
            ExprKind::Binary { left, right, .. } => {
                left.contains_next_variable() || right.contains_next_variable()
            }
            ExprKind::Integer { .. } | ExprKind::Boolean { .. } | ExprKind::Variable { .. } => {
                false
            }
        }
    }

    /// Rewrites every present-state variable into its post-state form.
    #[must_use]
    pub fn to_post_state(&self) -> Self {
        let kind = match &self.kind {
            ExprKind::Integer { value } => ExprKind::Integer { value: *value },
            ExprKind::Boolean { value } => ExprKind::Boolean { value: *value },
            ExprKind::Variable { name } | ExprKind::NextVariable { name } => {
                ExprKind::NextVariable { name: name.clone() }
            }
            ExprKind::Unary {
                operator,
                expression,
            } => ExprKind::Unary {
                operator: *operator,
                expression: Box::new(expression.to_post_state()),
            },
            ExprKind::Binary {
                operator,
                left,
                right,
            } => ExprKind::Binary {
                operator: *operator,
                left: Box::new(left.to_post_state()),
                right: Box::new(right.to_post_state()),
            },
        };
        Self {
            kind,
            span: self.span,
        }
    }
}
