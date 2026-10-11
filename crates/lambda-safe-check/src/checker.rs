Warning: truncated output (original token count: 12883)
Total output lines: 1468

use crate::core::{
    CanonicalHardInput, CheckedContract, CoreClaim, CoreSymbol, CoreTransition,
    CoreTransitionBranch, CoreType, StaticFailure, claim_class_name,
};
use crate::{StrategyDefinition, builtin_strategies};
use lambda_safe_model::{
    AuthorityClass, BinaryOperator, CONTRACT_SCHEMA_VERSION, ClaimClass, CompileStatus,
    ContractDocument, Diagnostic, ExcludedClaim, Expr, ExprKind, LANGUAGE_VERSION, SourceSpan,
    TransitionKind, UnaryOperator, ValueType, codes,
};
use std::collections::{BTreeMap, BTreeSet};

/// Static-check resource limits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckConfig {
    /// Maximum total declarations and operations.
    pub max_declarations: usize,
    /// Maximum nodes in any expression.
    pub max_expression_nodes: usize,
    /// Maximum checked expression-tree depth.
    pub max_expression_depth: usize,
    /// Largest accepted cumulative unit scale.
    pub max_unit_scale: i64,
}

impl Default for CheckConfig {
    fn default() -> Self {
        Self {
            max_declarations: 4_096,
            max_expression_nodes: 16_384,
            max_expression_depth: 256,
            max_unit_scale: 1_000_000_000_000,
        }
    }
}

/// Static failure type returned by [`check_contract`].
pub type CheckFailure = StaticFailure;

/// Applies version, naming, type, unit, authority, fragment, strategy,
/// transition-shape and linear-token checks.
pub fn check_contract(
    document: &ContractDocument,
    config: &CheckConfig,
) -> Result<CheckedContract, CheckFailure> {
    check_versions(document)?;
    check_resource_limits(document, config)?;
    check_identifier(&document.contract_id, SourceSpan::unknown(), "contract")?;
    check_contract_version(&document.contract_version)?;

    let units = resolve_units(document, config)?;
    let (environment, symbols) = build_environment(document, &units)?;
    check_unique_claim_ids(document)?;
    check_linear_tokens(document)?;

    let mut hard_assumptions = Vec::new();
    let mut hard_invariants = Vec::new();
    let mut excluded_claims = Vec::new();

    for assumption in &document.assumptions {
        check_identifier(&assumption.id, assumption.span, "assumption")?;
        let typed = lower_expression(&assumption.expression, &environment, false)?;
        require_boolean(&typed, assumption.expression.span, &assumption.id)?;
        match assumption.class {
            ClaimClass::Hard => {
                ensure_hard_authority(&assumption.expression, &environment, &assumption.id)?;
                hard_assumptions.push(CoreClaim {
                    id: assumption.id.clone(),
                    canonical_expression: canonical_expression(&typed.expression),
                    expression: typed.expression,
                });
            }
            class => excluded_claims.push(ExcludedClaim {
                id: assumption.id.clone(),
                class: claim_class_name(class).to_owned(),
                reason: "non-hard assumption is excluded from the hard context Γ".to_owned(),
            }),
        }
    }

    for invariant in &document.invariants {
        check_identifier(&invariant.id, invariant.span, "invariant")?;
        let typed = lower_expression(&invariant.expression, &environment, false)?;
        require_boolean(&typed, invariant.expression.span, &invariant.id)?;
        match invariant.class {
            ClaimClass::Hard => {
                ensure_hard_authority(&invariant.expression, &environment, &invariant.id)?;
                hard_invariants.push(CoreClaim {
                    id: invariant.id.clone(),
                    canonical_expression: canonical_expression(&typed.expression),
                    expression: typed.expression,
                });
            }
            class => excluded_claims.push(ExcludedClaim {
                id: invariant.id.clone(),
                class: claim_class_name(class).to_owned(),
                reason: match class {
                    ClaimClass::Trusted => {
                        "trusted-report claim is recorded but never elevated into hard output"
                    }
                    ClaimClass::Soft => "soft or predictive claim is outside hard certification",
                    ClaimClass::Hard => unreachable!("hard claims are handled separately"),
                }
                .to_owned(),
            }),
        }
    }

    if hard_invariants.is_empty() {
        return Err(failure(
            CompileStatus::Unsupported,
            Diagnostic::error(
                codes::OUTSIDE_FRAGMENT,
                "a certifying contract must declare at least one hard invariant",
            )
            .with_help("Declare a QF-LIA invariant with class `hard`."),
        ));
    }

    hard_assumptions.sort_by(|left, right| left.id.cmp(&right.id));
    hard_invariants.sort_by(|left, right| left.id.cmp(&right.id));
    excluded_claims.sort_by(|left, right| left.id.cmp(&right.id));

    let strategies = builtin_strategies();
    let transitions = check_transitions(document, &environment, &hard_invariants, &strategies)?;
    let mut linear_tokens = document
        .tokens
        .iter()
        .map(|token| token.name.clone())
        .collect::<Vec<_>>();
    linear_tokens.sort();

    let mut hard_symbol_names = BTreeSet::new();
    for claim in hard_assumptions.iter().chain(&hard_invariants) {
        hard_symbol_names.extend(claim.expression.referenced_variables());
    }
    for transition in &transitions {
        for branch in &transition.branches {
            hard_symbol_names.extend(branch.relation.referenced_variables());
        }
    }
    let hard_symbols = symbols
        .into_iter()
        .filter(|(name, _)| hard_symbol_names.contains(name))
        .collect::<BTreeMap<_, _>>();

    let hard_input = CanonicalHardInput {
        language_version: document.language_version.clone(),
        contract_id: document.contract_id.clone(),
        contract_version: document.contract_version.clone(),
        symbols: hard_symbols.values().cloned().collect(),
        assumptions: hard_assumptions.clone(),
        invariants: hard_invariants.clone(),
        transitions: transitions.clone(),
        linear_tokens: linear_tokens.clone(),
    };

    Ok(CheckedContract {
        contract_id: document.contract_id.clone(),
        contract_version: document.contract_version.clone(),
        language_version: document.language_version.clone(),
        symbols: hard_symbols,
        assumptions: hard_assumptions,
        invariants: hard_invariants,
        transitions,
        linear_tokens,
        strategies,
        excluded_claims,
        hard_input,
    })
}

fn check_versions(document: &ContractDocument) -> Result<(), CheckFailure> {
    if document.schema_version != CONTRACT_SCHEMA_VERSION {
        return Err(failure(
            CompileStatus::Unsupported,
            Diagnostic::error(
                codes::VERSION_MISMATCH,
                format!(
                    "contract schema version `{}` is unsupported; expected `{CONTRACT_SCHEMA_VERSION}`",
                    document.schema_version
                ),
            ),
        ));
    }
    if document.language_version != LANGUAGE_VERSION {
        return Err(failure(
            CompileStatus::Unsupported,
            Diagnostic::error(
                codes::VERSION_MISMATCH,
                format!(
                    "language version `{}` is unsupported; expected `{LANGUAGE_VERSION}`",
                    document.language_version
                ),
            ),
        ));
    }
    Ok(())
}

fn check_resource_limits(
    document: &ContractDocument,
    config: &CheckConfig,
) -> Result<(), CheckFailure> {
    let base_counts = [
        document.units.len(),
        document.constants.len(),
        document.symbols.len(),
        document.assumptions.len(),
        document.invariants.len(),
        document.transitions.len(),
        document.tokens.len(),
        document.consumes.len(),
        document.metadata.len(),
    ];
    let count = base_counts
        .into_iter()
        .chain(
            document
                .transitions
                .iter()
                .flat_map(|transition| [transition.branches.len(), transition.preserves.len()]),
        )
        .try_fold(0_usize, usize::checked_add)
        .ok_or_else(|| {
            failure(
                CompileStatus::Indeterminate,
                Diagnostic::error(
                    codes::RESOURCE_LIMIT,
                    "contract declaration count overflowed the platform size type",
                ),
            )
        })?;
    if count > config.max_declarations {
        return Err(failure(
            CompileStatus::Indeterminate,
            Diagnostic::error(
                codes::RESOURCE_LIMIT,
                format!(
                    "contract contains {count} declarations; the configured limit is {}",
                    config.max_declarations
                ),
            ),
        ));
    }

    for (subject, expression) in all_expressions(document) {
        let (nodes, depth) = expression_profile(expression);
        if nodes > config.max_expression_nodes {
            return Err(failure(
                CompileStatus::Indeterminate,
                Diagnostic::error(
                    codes::RESOURCE_LIMIT,
                    format!(
                        "expression `{subject}` contains {nodes} nodes; the configured limit is {}",
                        config.max_expression_nodes
                    ),
                )
                .at(expression.span)
                .for_subject(subject),
            ));
        }
        if depth > config.max_expression_depth {
            return Err(failure(
                CompileStatus::Indeterminate,
                Diagnostic::error(
                    codes::RESOURCE_LIMIT,
                    format!(
                        "expression `{subject}` has tree depth {depth}; the configured limit is {}",
                        config.max_expression_depth
                    ),
                )
                .at(expression.span)
                .for_subject(subject),
            ));
        }
    }
    Ok(())
}

fn all_expressions(document: &ContractDocument) -> Vec<(&str, &Expr)> {
    let mut expressions = Vec::new();
    for assumption in &document.assumptions {
        expressions.push((assumption.id.as_str(), &assumption.expression));
    }
    for invariant in &document.invariants {
        expressions.push((invariant.id.as_str(), &invariant.expression));
    }
    for transition in &document.transitions {
        for branch in &transition.branches {
            expressions.push((transition.id.as_str(), &branch.relation));
        }
    }
    expressions
}

fn expression_profile(expression: &Expr) -> (usize, usize) {
    let mut nodes = 0_usize;
    let mut maximum_depth = 0_usize;
    let mut pending = vec![(expression, 1_usize)];
    while let Some((current, depth)) = pending.pop() {
        nodes = nodes.saturating_add(1);
        maximum_depth = maximum_depth.max(depth);
        match &current.kind {
            ExprKind::Unary { expression, .. } => pending.push((expression, depth + 1)),
            ExprKind::Binary { left, right, .. } => {
                pending.push((left, depth + 1));
                pending.push((right, depth + 1));
            }
            ExprKind::Integer { .. }
            | ExprKind::Boolean { .. }
            | ExprKind::Variable { .. }
            | ExprKind::NextVariable { .. } => {}
        }
    }
    (nodes, maximum_depth)
}

#[derive(Clone, Debug)]
struct ResolvedUnit {
    base: String,
    scale: i64,
}

fn resolve_units(
    document: &ContractDocument,
    config: &CheckConfig,
) -> Result<BTreeMap<String, ResolvedUnit>, CheckFailure> {
    let mut definitions = BTreeMap::new();
    let _ = definitions.insert(
        "unitless".to_owned(),
        ("unitless".to_owned(), 1_i64, SourceSpan::unknown()),
    );
    for unit in &document.units {
        check_identifier(&unit.name, unit.span, "unit")?;
        check_identifier(&unit.base, unit.span, "base unit")?;
        if unit.scale <= 0 {
            return Err(failure(
                CompileStatus::MalformedContract,
                Diagnostic::error(codes::UNIT_MISMATCH, "unit scale must be positive")
                    .at(unit.span)
                    .for_subject(&unit.name),
            ));
        }
        if definitions
            .insert(
                unit.name.clone(),
                (unit.base.clone(), unit.scale, unit.span),
            )
            .is_some()
        {
            return Err(duplicate(&unit.name, unit.span));
        }
    }

    let mut resolved = BTreeMap::new();
    let names = definitions.keys().cloned().collect::<Vec<_>>();
    for name in names {
        let mut visiting = BTreeSet::new();
        resolve_unit(
            &name,
            &definitions,
            &mut resolved,
            &mut visiting,
            config.max_unit_scale,
        )?;
    }
    Ok(resolved)
}

fn resolve_unit(
    name: &str,
    definitions: &BTreeMap<String, (String, i64, SourceSpan)>,
    resolved: &mut BTreeMap<String, ResolvedUnit>,
    visiting: &mut BTreeSet<String>,
    max_scale: i64,
) -> Result<ResolvedUnit, CheckFailure> {
    const MAX_UNIT_CHAIN_DEPTH: usize = 256;
    if let Some(unit) = resolved.get(name) {
        return Ok(unit.clone());
    }
    let Some((base, scale, span)) = definitions.get(name) else {
        return Err(failure(
            CompileStatus::MalformedContract,
            Diagnostic::error(codes::UNIT_MISMATCH, format!("unknown unit `{name}`")),
        ));
    };
    if visiting.len() >= MAX_UNIT_CHAIN_DEPTH {
        return Err(failure(
            CompileStatus::Indeterminate,
            Diagnostic::error(
                codes::RESOURCE_LIMIT,
                format!("unit alias chain exceeds the supported depth of {MAX_UNIT_CHAIN_DEPTH}"),
            )
            .at(*span)
            .for_subject(name),
        ));
    }
    if !visiting.insert(name.to_owned()) {
        return Err(failure(
            CompileStatus::MalformedContract,
            Diagnostic::error(codes::UNIT_MISMATCH, "cyclic unit definition")
                .at(*span)
                .for_subject(name),
        ));
    }

    let unit = if base == name {
        if *scale != 1 {
            return Err(failure(
                CompileStatus::MalformedContract,
                Diagnostic::error(codes::UNIT_MISMATCH, "a base unit must have scale one")
                    .at(*span)
                    .for_subject(name),
            ));
        }
        ResolvedUnit {
            base: name.to_owned(),
            scale: 1,
        }
    } else {
        if !definitions.contains_key(base) {
            return Err(failure(
                CompileStatus::MalformedContract,
                Diagnostic::error(
                    codes::UNIT_MISMATCH,
                    format!("unit `{name}` references undeclared base unit `{base}`"),
                )
                .at(*span)
                .for_subject(name),
            ));
        }
        let parent = resolve_unit(base, definitions, resolved, visiting, max_scale)?;
        let total = scale.checked_mul(parent.scale).ok_or_else(|| {
            failure(
                CompileStatus::Unsupported,
                Diagnostic::error(codes::UNIT_MISMATCH, "unit scale multiplication overflowed")
                    .at(*span)
                    .for_subject(name),
            )
        })?;
        if total > max_scale {
            return Err(failure(
                CompileStatus::Unsupported,
                Diagnostic::error(
                    codes::UNIT_MISMATCH,
                    format!("canonical unit scale {total} exceeds configured limit {max_scale}"),
                )
                .at(*span)
                .for_subject(n…4883 tokens truncated… CheckFailure> {
    use BinaryOperator::{
        Add, And, Equal, GreaterThan, GreaterThanOrEqual, Implies, LessThan, LessThanOrEqual,
        Multiply, NotEqual, Or, Subtract,
    };

    let (value_type, constant_value) = match operator {
        And | Or | Implies => {
            if left.value_type != SemanticType::Boolean || right.value_type != SemanticType::Boolean
            {
                return Err(type_error(span, "boolean operands", "integer operand"));
            }
            (SemanticType::Boolean, None)
        }
        Add | Subtract => {
            let result_type = compatible_integer_type(&left, &right, span)?;
            let value = match (left.constant_value, right.constant_value) {
                (Some(left), Some(right)) if operator == Add => left.checked_add(right),
                (Some(left), Some(right)) => left.checked_sub(right),
                _ => None,
            };
            (result_type, value)
        }
        Multiply => {
            let left_integer = matches!(&left.value_type, SemanticType::Integer { .. });
            let right_integer = matches!(&right.value_type, SemanticType::Integer { .. });
            if !left_integer || !right_integer {
                return Err(type_error(span, "integer operands", "boolean operand"));
            }
            let left_scalar = matches!(&left.value_type, SemanticType::Integer { base: None })
                && left.constant_value.is_some();
            let right_scalar = matches!(&right.value_type, SemanticType::Integer { base: None })
                && right.constant_value.is_some();
            if !left_scalar && !right_scalar {
                return Err(failure(
                    CompileStatus::Unsupported,
                    Diagnostic::error(
                        codes::OUTSIDE_FRAGMENT,
                        "non-linear multiplication is outside QF-LIA; one operand must be a dimensionless compile-time integer",
                    )
                    .at(span),
                ));
            }
            let result_type = if left_scalar {
                right.value_type.clone()
            } else {
                left.value_type.clone()
            };
            let value = match (left.constant_value, right.constant_value) {
                (Some(left), Some(right)) => left.checked_mul(right),
                _ => None,
            };
            (result_type, value)
        }
        Equal | NotEqual => {
            match (&left.value_type, &right.value_type) {
                (SemanticType::Boolean, SemanticType::Boolean) => {}
                (SemanticType::Integer { .. }, SemanticType::Integer { .. }) => {
                    compatible_integer_type(&left, &right, span)?;
                }
                _ => return Err(type_error(span, "operands of the same sort", "mixed sorts")),
            }
            (SemanticType::Boolean, None)
        }
        LessThan | LessThanOrEqual | GreaterThan | GreaterThanOrEqual => {
            compatible_integer_type(&left, &right, span)?;
            (SemanticType::Boolean, None)
        }
    };

    Ok(TypedExpression {
        expression: Expr {
            kind: ExprKind::Binary {
                operator,
                left: Box::new(left.expression),
                right: Box::new(right.expression),
            },
            span,
        },
        value_type,
        constant_value,
    })
}

fn compatible_integer_type(
    left: &TypedExpression,
    right: &TypedExpression,
    span: SourceSpan,
) -> Result<SemanticType, CheckFailure> {
    let SemanticType::Integer { base: left_base } = &left.value_type else {
        return Err(type_error(span, "integer operands", "boolean operand"));
    };
    let SemanticType::Integer { base: right_base } = &right.value_type else {
        return Err(type_error(span, "integer operands", "boolean operand"));
    };
    if left_base == right_base {
        return Ok(left.value_type.clone());
    }
    if left_base.is_none() && left.constant_value == Some(0) {
        return Ok(right.value_type.clone());
    }
    if right_base.is_none() && right.constant_value == Some(0) {
        return Ok(left.value_type.clone());
    }
    Err(failure(
        CompileStatus::MalformedContract,
        Diagnostic::error(
            codes::UNIT_MISMATCH,
            format!(
                "incompatible units `{}` and `{}`",
                left_base.as_deref().unwrap_or("unitless"),
                right_base.as_deref().unwrap_or("unitless")
            ),
        )
        .at(span),
    ))
}

fn require_boolean(
    expression: &TypedExpression,
    span: SourceSpan,
    subject: &str,
) -> Result<(), CheckFailure> {
    if expression.value_type == SemanticType::Boolean {
        Ok(())
    } else {
        Err(failure(
            CompileStatus::MalformedContract,
            Diagnostic::error(
                codes::TYPE_MISMATCH,
                "claim expression must evaluate to boolean",
            )
            .at(span)
            .for_subject(subject),
        ))
    }
}

fn ensure_hard_authority(
    expression: &Expr,
    environment: &BTreeMap<String, Binding>,
    subject: &str,
) -> Result<(), CheckFailure> {
    for name in expression.referenced_variables() {
        let Some(binding) = environment.get(&name) else {
            return Err(failure(
                CompileStatus::MalformedContract,
                Diagnostic::error(codes::UNKNOWN_NAME, format!("unknown identifier `{name}`"))
                    .for_subject(subject),
            ));
        };
        if !binding.authority.hard_eligible() {
            return Err(failure(
                CompileStatus::AuthorityViolation,
                Diagnostic::error(
                    codes::AUTHORITY_VIOLATION,
                    format!(
                        "hard expression `{subject}` depends on `{name}` with authority `{:?}`",
                        binding.authority
                    ),
                )
                .at(expression.span)
                .for_subject(subject)
                .with_help("Reclassify the claim as trusted/soft or ground the operand in control-plane authority."),
            ));
        }
    }
    Ok(())
}

/// Unambiguous prefix notation used in canonical artifacts.
#[must_use]
pub fn canonical_expression(expression: &Expr) -> String {
    match &expression.kind {
        ExprKind::Integer { value } => value.to_string(),
        ExprKind::Boolean { value } => value.to_string(),
        ExprKind::Variable { name } => name.clone(),
        ExprKind::NextVariable { name } => format!("(next {name})"),
        ExprKind::Unary {
            operator,
            expression,
        } => format!(
            "({} {})",
            unary_name(*operator),
            canonical_expression(expression)
        ),
        ExprKind::Binary {
            operator,
            left,
            right,
        } => format!(
            "({} {} {})",
            binary_name(*operator),
            canonical_expression(left),
            canonical_expression(right)
        ),
    }
}

const fn unary_name(operator: UnaryOperator) -> &'static str {
    match operator {
        UnaryOperator::Not => "not",
        UnaryOperator::Negate => "neg",
    }
}

const fn binary_name(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Add => "+",
        BinaryOperator::Subtract => "-",
        BinaryOperator::Multiply => "*",
        BinaryOperator::Equal => "=",
        BinaryOperator::NotEqual => "!=",
        BinaryOperator::LessThan => "<",
        BinaryOperator::LessThanOrEqual => "<=",
        BinaryOperator::GreaterThan => ">",
        BinaryOperator::GreaterThanOrEqual => ">=",
        BinaryOperator::And => "and",
        BinaryOperator::Or => "or",
        BinaryOperator::Implies => "=>",
    }
}

fn check_identifier(name: &str, span: SourceSpan, kind: &str) -> Result<(), CheckFailure> {
    if is_valid_identifier(name) {
        Ok(())
    } else {
        Err(failure(
            CompileStatus::MalformedContract,
            Diagnostic::error(
                codes::INVALID_SOURCE,
                format!(
                    "invalid or reserved {kind} identifier `{name}`; use 1-128 ASCII letters, digits or underscores, do not start with a digit, avoid language keywords, and do not use the internal `__next` suffix"
                ),
            )
            .at(span)
            .for_subject(name),
        ))
    }
}

/// Whether a name is a legal user-defined λ-Safe identifier.
#[must_use]
pub fn is_valid_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|value| value.is_ascii_alphabetic() || value == '_')
        && characters.all(|value| value.is_ascii_alphanumeric() || value == '_')
        && name.len() <= 128
        && !is_reserved_identifier(name)
        && !name.ends_with("__next")
}

fn check_contract_version(version: &str) -> Result<(), CheckFailure> {
    if !version.is_empty()
        && version.len() <= 128
        && version.chars().all(|value| !value.is_control())
    {
        Ok(())
    } else {
        Err(failure(
            CompileStatus::MalformedContract,
            Diagnostic::error(
                codes::INVALID_SOURCE,
                "contract version must contain 1-128 non-control characters",
            ),
        ))
    }
}

fn is_reserved_identifier(name: &str) -> bool {
    matches!(
        name,
        "lambda_safe"
            | "contract"
            | "version"
            | "unit"
            | "unitless"
            | "const"
            | "input"
            | "bool"
            | "int"
            | "authority"
            | "control_plane"
            | "constant"
            | "trusted_report"
            | "predictive"
            | "unsupported"
            | "range"
            | "assume"
            | "invariant"
            | "hard"
            | "trusted"
            | "soft"
            | "degrade"
            | "reconcile"
            | "preserves"
            | "primary"
            | "fallback"
            | "merge"
            | "using"
            | "token"
            | "consume"
            | "metadata"
            | "next"
            | "true"
            | "false"
    )
}

fn type_error(span: SourceSpan, expected: &str, actual: &str) -> CheckFailure {
    failure(
        CompileStatus::MalformedContract,
        Diagnostic::error(
            codes::TYPE_MISMATCH,
            format!("type mismatch: expected {expected}, found {actual}"),
        )
        .at(span),
    )
}

fn duplicate(name: &str, span: SourceSpan) -> CheckFailure {
    failure(
        CompileStatus::MalformedContract,
        Diagnostic::error(
            codes::DUPLICATE_NAME,
            format!("duplicate identifier `{name}`"),
        )
        .at(span)
        .for_subject(name),
    )
}

fn failure(status: CompileStatus, diagnostic: Diagnostic) -> CheckFailure {
    CheckFailure {
        status,
        diagnostics: vec![diagnostic],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_safe_model::{
        AssumptionDefinition, ConstantDefinition, InvariantDefinition, SymbolDefinition,
        UnitDefinition,
    };
    use lambda_safe_syntax::parse_expression;

    fn base_contract() -> ContractDocument {
        ContractDocument {
            schema_version: CONTRACT_SCHEMA_VERSION.to_owned(),
            language_version: LANGUAGE_VERSION.to_owned(),
            contract_id: "capacity".to_owned(),
            contract_version: "1.0.0".to_owned(),
            units: vec![UnitDefinition {
                name: "cpu".to_owned(),
                base: "cpu".to_owned(),
                scale: 1,
                span: SourceSpan::unknown(),
            }],
            constants: vec![ConstantDefinition {
                name: "zero".to_owned(),
                value: 0,
                unit: "cpu".to_owned(),
                span: SourceSpan::unknown(),
            }],
            symbols: vec![
                SymbolDefinition {
                    name: "allocated".to_owned(),
                    value_type: ValueType::integer("cpu"),
                    authority: AuthorityClass::ControlPlane,
                    minimum: Some(0),
                    maximum: Some(10),
                    provenance: None,
                    span: SourceSpan::unknown(),
                },
                SymbolDefinition {
                    name: "capacity".to_owned(),
                    value_type: ValueType::integer("cpu"),
                    authority: AuthorityClass::ControlPlane,
                    minimum: Some(0),
                    maximum: Some(10),
                    provenance: None,
                    span: SourceSpan::unknown(),
                },
            ],
            assumptions: vec![AssumptionDefinition {
                id: "bounded".to_owned(),
                class: ClaimClass::Hard,
                expression: parse_expression("allocated <= capacity")
                    .expect("test expression should parse"),
                span: SourceSpan::unknown(),
            }],
            invariants: vec![InvariantDefinition {
                id: "within_capacity".to_owned(),
                class: ClaimClass::Hard,
                expression: parse_expression("allocated <= capacity")
                    .expect("test expression should parse"),
                span: SourceSpan::unknown(),
            }],
            transitions: Vec::new(),
            tokens: Vec::new(),
            consumes: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn accepts_well_typed_hard_contract() {
        let checked = check_contract(&base_contract(), &CheckConfig::default());
        assert!(checked.is_ok());
    }

    #[test]
    fn rejects_trusted_report_in_hard_claim() {
        let mut contract = base_contract();
        contract.symbols[0].authority = AuthorityClass::TrustedReport;
        let failure = check_contract(&contract, &CheckConfig::default())
            .expect_err("authority crossing must fail");
        assert_eq!(failure.status, CompileStatus::AuthorityViolation);
    }

    #[test]
    fn rejects_non_linear_hard_expression() {
        let mut contract = base_contract();
        contract.invariants[0].expression = parse_expression("allocated * capacity <= capacity")
            .expect("test expression should parse");
        let failure = check_contract(&contract, &CheckConfig::default())
            .expect_err("non-linear multiplication must fail");
        assert_eq!(failure.status, CompileStatus::Unsupported);
    }

    #[test]
    fn accepts_named_dimensionless_constant_multiplication() {
        let mut contract = base_contract();
        contract.constants.push(ConstantDefinition {
            name: "factor".to_owned(),
            value: 1,
            unit: "unitless".to_owned(),
            span: SourceSpan::unknown(),
        });
        contract.invariants[0].expression = parse_expression("factor * allocated <= capacity")
            .expect("test expression should parse");
        assert!(check_contract(&contract, &CheckConfig::default()).is_ok());
    }

    #[test]
    fn rejects_deep_flat_expression_before_recursive_lowering() {
        let mut contract = base_contract();
        let expression = std::iter::repeat_n("allocated", 300)
            .collect::<Vec<_>>()
            .join(" + ");
        contract.invariants[0].expression = parse_expression(&format!("{expression} <= capacity"))
            .expect("flat test expression should parse iteratively");
        let failure = check_contract(&contract, &CheckConfig::default())
            .expect_err("deep expression tree must fail before lowering");
        assert_eq!(failure.status, CompileStatus::Indeterminate);
        assert_eq!(failure.diagnostics[0].code, codes::RESOURCE_LIMIT);
    }
}

