use crate::core::{
    CanonicalHardInput, CheckedContract, CoreClaim, CoreSymbol, CoreTransition,
    CoreTransitionBranch, CoreType, StaticFailure, claim_class_name,
};
use crate::{StrategyDefinition, builtin_strategies};
use lambda_safe_model::{
    AuthorityClass, BinaryOperator, ClaimClass, CompileStatus, ContractDocument, Diagnostic,
    ExcludedClaim, Expr, ExprKind, SourceSpan, TransitionKind, UnaryOperator, ValueType, codes,
    CONTRACT_SCHEMA_VERSION, LANGUAGE_VERSION,
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
                    ClaimClass::Soft => {
                        "soft or predictive claim is outside hard certification"
                    }
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
                format!(
                    "unit alias chain exceeds the supported depth of {MAX_UNIT_CHAIN_DEPTH}"
                ),
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
                Diagnostic::error(
                    codes::UNIT_MISMATCH,
                    "a base unit must have scale one",
                )
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
                .for_subject(name),
            ));
        }
        ResolvedUnit {
            base: parent.base,
            scale: total,
        }
    };
    let _ = visiting.remove(name);
    let _ = resolved.insert(name.to_owned(), unit.clone());
    Ok(unit)
}

#[derive(Clone, Debug)]
struct Binding {
    value_type: SemanticType,
    authority: AuthorityClass,
    constant: Option<i64>,
    nominal_scale: i64,
    is_runtime_symbol: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SemanticType {
    Boolean,
    Integer { base: Option<String> },
}

fn build_environment(
    document: &ContractDocument,
    units: &BTreeMap<String, ResolvedUnit>,
) -> Result<(BTreeMap<String, Binding>, BTreeMap<String, CoreSymbol>), CheckFailure> {
    let mut environment = BTreeMap::new();
    let mut symbols = BTreeMap::new();

    for constant in &document.constants {
        check_identifier(&constant.name, constant.span, "constant")?;
        let unit = units.get(&constant.unit).ok_or_else(|| {
            failure(
                CompileStatus::MalformedContract,
                Diagnostic::error(
                    codes::UNIT_MISMATCH,
                    format!("constant `{}` uses unknown unit `{}`", constant.name, constant.unit),
                )
                .at(constant.span)
                .for_subject(&constant.name),
            )
        })?;
        let value = constant.value.checked_mul(unit.scale).ok_or_else(|| {
            failure(
                CompileStatus::Unsupported,
                Diagnostic::error(
                    codes::UNIT_MISMATCH,
                    format!("constant `{}` overflows during unit normalization", constant.name),
                )
                .at(constant.span),
            )
        })?;
        let binding = Binding {
            value_type: SemanticType::Integer {
                base: (unit.base != "unitless").then(|| unit.base.clone()),
            },
            authority: AuthorityClass::Constant,
            constant: Some(value),
            nominal_scale: unit.scale,
            is_runtime_symbol: false,
        };
        if environment.insert(constant.name.clone(), binding).is_some() {
            return Err(duplicate(&constant.name, constant.span));
        }
    }

    for symbol in &document.symbols {
        check_identifier(&symbol.name, symbol.span, "input")?;
        if symbol.authority == AuthorityClass::Constant {
            return Err(failure(
                CompileStatus::MalformedContract,
                Diagnostic::error(
                    codes::AUTHORITY_VIOLATION,
                    "runtime inputs cannot use `constant` authority",
                )
                .at(symbol.span)
                .for_subject(&symbol.name)
                .with_help("Use a `const` declaration for compile-time constants."),
            ));
        }
        let (semantic_type, core_type, scale) = match &symbol.value_type {
            ValueType::Boolean => {
                if symbol.minimum.is_some() || symbol.maximum.is_some() {
                    return Err(failure(
                        CompileStatus::MalformedContract,
                        Diagnostic::error(codes::TYPE_MISMATCH, "boolean inputs cannot have integer ranges")
                            .at(symbol.span)
                            .for_subject(&symbol.name),
                    ));
                }
                (SemanticType::Boolean, CoreType::Boolean, 1)
            }
            ValueType::Integer { unit } => {
                let resolved = units.get(unit).ok_or_else(|| {
                    failure(
                        CompileStatus::MalformedContract,
                        Diagnostic::error(
                            codes::UNIT_MISMATCH,
                            format!("input `{}` uses unknown unit `{unit}`", symbol.name),
                        )
                        .at(symbol.span)
                        .for_subject(&symbol.name),
                    )
                })?;
                if matches!((symbol.minimum, symbol.maximum), (Some(min), Some(max)) if min > max) {
                    return Err(failure(
                        CompileStatus::MalformedContract,
                        Diagnostic::error(codes::TYPE_MISMATCH, "input minimum is greater than maximum")
                            .at(symbol.span)
                            .for_subject(&symbol.name),
                    ));
                }
                (
                    SemanticType::Integer {
                        base: (resolved.base != "unitless")
                            .then(|| resolved.base.clone()),
                    },
                    CoreType::Integer {
                        base_unit: resolved.base.clone(),
                        scale: resolved.scale,
                    },
                    resolved.scale,
                )
            }
        };
        let binding = Binding {
            value_type: semantic_type,
            authority: symbol.authority,
            constant: None,
            nominal_scale: scale,
            is_runtime_symbol: true,
        };
        if environment.insert(symbol.name.clone(), binding).is_some() {
            return Err(duplicate(&symbol.name, symbol.span));
        }
        let _ = symbols.insert(
            symbol.name.clone(),
            CoreSymbol {
                name: symbol.name.clone(),
                value_type: core_type,
                authority: symbol.authority,
                minimum: symbol.minimum,
                maximum: symbol.maximum,
            },
        );
    }
    Ok((environment, symbols))
}

fn check_unique_claim_ids(document: &ContractDocument) -> Result<(), CheckFailure> {
    let mut ids = BTreeSet::new();
    for (id, span) in document
        .assumptions
        .iter()
        .map(|value| (&value.id, value.span))
        .chain(document.invariants.iter().map(|value| (&value.id, value.span)))
        .chain(document.transitions.iter().map(|value| (&value.id, value.span)))
    {
        if !ids.insert(id.clone()) {
            return Err(duplicate(id, span));
        }
    }
    Ok(())
}

fn check_linear_tokens(document: &ContractDocument) -> Result<(), CheckFailure> {
    let mut declared = BTreeSet::new();
    for token in &document.tokens {
        check_identifier(&token.name, token.span, "token")?;
        if !declared.insert(token.name.clone()) {
            return Err(duplicate(&token.name, token.span));
        }
    }
    let mut counts = BTreeMap::<String, usize>::new();
    for consumption in &document.consumes {
        if !declared.contains(&consumption.name) {
            return Err(failure(
                CompileStatus::MalformedContract,
                Diagnostic::error(
                    codes::LINEAR_TOKEN_ERROR,
                    format!("unknown linear token `{}`", consumption.name),
                )
                .at(consumption.span)
                .for_subject(&consumption.name),
            ));
        }
        *counts.entry(consumption.name.clone()).or_default() += 1;
    }
    for token in declared {
        let count = counts.get(&token).copied().unwrap_or_default();
        if count != 1 {
            return Err(failure(
                CompileStatus::MalformedContract,
                Diagnostic::error(
                    codes::LINEAR_TOKEN_ERROR,
                    format!("linear token `{token}` must be consumed exactly once; observed {count}"),
                )
                .for_subject(token),
            ));
        }
    }
    Ok(())
}

fn check_transitions(
    document: &ContractDocument,
    environment: &BTreeMap<String, Binding>,
    hard_invariants: &[CoreClaim],
    strategies: &[StrategyDefinition],
) -> Result<Vec<CoreTransition>, CheckFailure> {
    let hard_ids = hard_invariants
        .iter()
        .map(|claim| claim.id.as_str())
        .collect::<BTreeSet<_>>();
    let strategy_ids = strategies
        .iter()
        .map(|strategy| strategy.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut checked = Vec::new();

    for transition in &document.transitions {
        check_identifier(&transition.id, transition.span, "transition")?;
        if transition.preserves.is_empty() {
            return Err(transition_failure(
                transition.kind,
                &transition.id,
                transition.span,
                "transition must name at least one hard invariant to preserve",
            ));
        }
        let mut preserves = transition.preserves.clone();
        preserves.sort();
        let original_preserve_count = preserves.len();
        preserves.dedup();
        if preserves.len() != original_preserve_count {
            return Err(transition_failure(
                transition.kind,
                &transition.id,
                transition.span,
                "transition preservation list contains a duplicate invariant",
            ));
        }
        for invariant in &preserves {
            if !hard_ids.contains(invariant.as_str()) {
                return Err(transition_failure(
                    transition.kind,
                    &transition.id,
                    transition.span,
                    format!("`{invariant}` is not a declared hard invariant"),
                ));
            }
        }
        let preserve_ids = preserves
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if preserve_ids != hard_ids {
            let missing = hard_ids
                .difference(&preserve_ids)
                .copied()
                .collect::<Vec<_>>();
            return Err(transition_failure(
                transition.kind,
                &transition.id,
                transition.span,
                format!(
                    "transition must preserve every hard invariant; missing [{}]",
                    missing.join(", ")
                ),
            ));
        }

        match transition.kind {
            TransitionKind::Degrade => {
                if transition.strategy.is_some()
                    || transition.branches.len() != 2
                    || transition.branches[0].name != "primary"
                    || transition.branches[1].name != "fallback"
                {
                    return Err(transition_failure(
                        transition.kind,
                        &transition.id,
                        transition.span,
                        "degrade requires ordered `primary` and `fallback` branches and no strategy",
                    ));
                }
            }
            TransitionKind::Reconcile => {
                let Some(strategy) = &transition.strategy else {
                    return Err(transition_failure(
                        transition.kind,
                        &transition.id,
                        transition.span,
                        "reconcile requires a registered strategy",
                    ));
                };
                if !strategy_ids.contains(strategy.as_str()) {
                    return Err(transition_failure(
                        transition.kind,
                        &transition.id,
                        transition.span,
                        format!("reconciliation strategy `{strategy}` is not registered"),
                    ));
                }
                if transition.branches.len() != 1 || transition.branches[0].name != "merge" {
                    return Err(transition_failure(
                        transition.kind,
                        &transition.id,
                        transition.span,
                        "reconcile requires exactly one `merge` relation",
                    ));
                }
            }
        }

        let mut branches = Vec::new();
        for branch in &transition.branches {
            if !branch.relation.contains_next_variable() {
                return Err(transition_failure(
                    transition.kind,
                    &transition.id,
                    branch.span,
                    format!("branch `{}` does not constrain any post-state variable", branch.name),
                ));
            }
            ensure_hard_authority(&branch.relation, environment, &transition.id)?;
            let typed = lower_expression(&branch.relation, environment, true)?;
            require_boolean(&typed, branch.relation.span, &transition.id)?;
            branches.push(CoreTransitionBranch {
                name: branch.name.clone(),
                canonical_relation: canonical_expression(&typed.expression),
                relation: typed.expression,
            });
        }
        checked.push(CoreTransition {
            id: transition.id.clone(),
            kind: transition.kind,
            strategy: transition.strategy.clone(),
            preserves,
            branches,
        });
    }
    checked.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(checked)
}

fn transition_failure(
    kind: TransitionKind,
    id: &str,
    span: SourceSpan,
    message: impl Into<String>,
) -> CheckFailure {
    let code = match kind {
        TransitionKind::Degrade => codes::DEGRADE_VIOLATION,
        TransitionKind::Reconcile => codes::RECONCILE_VIOLATION,
    };
    failure(
        CompileStatus::Unsupported,
        Diagnostic::error(code, message).at(span).for_subject(id),
    )
}

#[derive(Clone, Debug)]
struct TypedExpression {
    expression: Expr,
    value_type: SemanticType,
    constant_value: Option<i64>,
}

fn lower_expression(
    expression: &Expr,
    environment: &BTreeMap<String, Binding>,
    allow_next: bool,
) -> Result<TypedExpression, CheckFailure> {
    match &expression.kind {
        ExprKind::Integer { value } => Ok(TypedExpression {
            expression: expression.clone(),
            value_type: SemanticType::Integer { base: None },
            constant_value: Some(*value),
        }),
        ExprKind::Boolean { .. } => Ok(TypedExpression {
            expression: expression.clone(),
            value_type: SemanticType::Boolean,
            constant_value: None,
        }),
        ExprKind::Variable { name } => lower_binding(name, false, expression.span, environment),
        ExprKind::NextVariable { name } => {
            if !allow_next {
                return Err(failure(
                    CompileStatus::Unsupported,
                    Diagnostic::error(
                        codes::OUTSIDE_FRAGMENT,
                        "post-state `next(...)` references are legal only in transition relations",
                    )
                    .at(expression.span)
                    .for_subject(name),
                ));
            }
            lower_binding(name, true, expression.span, environment)
        }
        ExprKind::Unary {
            operator,
            expression: inner,
        } => {
            let typed = lower_expression(inner, environment, allow_next)?;
            match operator {
                UnaryOperator::Not if typed.value_type == SemanticType::Boolean => {
                    Ok(TypedExpression {
                        expression: Expr {
                            kind: ExprKind::Unary {
                                operator: *operator,
                                expression: Box::new(typed.expression),
                            },
                            span: expression.span,
                        },
                        value_type: SemanticType::Boolean,
                        constant_value: None,
                    })
                }
                UnaryOperator::Negate => {
                    let SemanticType::Integer { .. } = &typed.value_type else {
                        return Err(type_error(expression.span, "integer", "boolean"));
                    };
                    let constant_value = typed.constant_value.and_then(i64::checked_neg);
                    Ok(TypedExpression {
                        expression: Expr {
                            kind: ExprKind::Unary {
                                operator: *operator,
                                expression: Box::new(typed.expression),
                            },
                            span: expression.span,
                        },
                        value_type: typed.value_type,
                        constant_value,
                    })
                }
                UnaryOperator::Not => Err(type_error(expression.span, "boolean", "integer")),
            }
        }
        ExprKind::Binary {
            operator,
            left,
            right,
        } => {
            let left = lower_expression(left, environment, allow_next)?;
            let right = lower_expression(right, environment, allow_next)?;
            lower_binary(*operator, left, right, expression.span)
        }
    }
}

fn lower_binding(
    name: &str,
    next: bool,
    span: SourceSpan,
    environment: &BTreeMap<String, Binding>,
) -> Result<TypedExpression, CheckFailure> {
    let binding = environment.get(name).ok_or_else(|| {
        failure(
            CompileStatus::MalformedContract,
            Diagnostic::error(codes::UNKNOWN_NAME, format!("unknown identifier `{name}`"))
                .at(span)
                .for_subject(name),
        )
    })?;
    if next && !binding.is_runtime_symbol {
        return Err(failure(
            CompileStatus::MalformedContract,
            Diagnostic::error(
                codes::TYPE_MISMATCH,
                format!("compile-time constant `{name}` has no post-state value"),
            )
            .at(span)
            .for_subject(name),
        ));
    }

    if let Some(value) = binding.constant {
        return Ok(TypedExpression {
            expression: Expr {
                kind: ExprKind::Integer { value },
                span,
            },
            value_type: binding.value_type.clone(),
            constant_value: Some(value),
        });
    }

    let variable = Expr {
        kind: if next {
            ExprKind::NextVariable {
                name: name.to_owned(),
            }
        } else {
            ExprKind::Variable {
                name: name.to_owned(),
            }
        },
        span,
    };
    let normalized = if binding.nominal_scale == 1
        || binding.value_type == SemanticType::Boolean
    {
        variable
    } else {
        Expr {
            kind: ExprKind::Binary {
                operator: BinaryOperator::Multiply,
                left: Box::new(Expr::integer(binding.nominal_scale)),
                right: Box::new(variable),
            },
            span,
        }
    };
    Ok(TypedExpression {
        expression: normalized,
        value_type: binding.value_type.clone(),
        constant_value: None,
    })
}

fn lower_binary(
    operator: BinaryOperator,
    left: TypedExpression,
    right: TypedExpression,
    span: SourceSpan,
) -> Result<TypedExpression, CheckFailure> {
    use BinaryOperator::{
        Add, And, Equal, GreaterThan, GreaterThanOrEqual, Implies, LessThan,
        LessThanOrEqual, Multiply, NotEqual, Or, Subtract,
    };

    let (value_type, constant_value) = match operator {
        And | Or | Implies => {
            if left.value_type != SemanticType::Boolean
                || right.value_type != SemanticType::Boolean
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
            Diagnostic::error(codes::TYPE_MISMATCH, "claim expression must evaluate to boolean")
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
        } => format!("({} {})", unary_name(*operator), canonical_expression(expression)),
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
        Diagnostic::error(codes::DUPLICATE_NAME, format!("duplicate identifier `{name}`"))
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
        contract.invariants[0].expression =
            parse_expression(&format!("{expression} <= capacity"))
                .expect("flat test expression should parse iteratively");
        let failure = check_contract(&contract, &CheckConfig::default())
            .expect_err("deep expression tree must fail before lowering");
        assert_eq!(failure.status, CompileStatus::Indeterminate);
        assert_eq!(failure.diagnostics[0].code, codes::RESOURCE_LIMIT);
    }
}
