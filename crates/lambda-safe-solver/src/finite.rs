use crate::query::{DecisionProcedure, DecisionResult, SolverError, SolverQuery};
use lambda_safe_check::{CoreSymbol, CoreType};
use lambda_safe_model::{
    BinaryOperator, Counterexample, Expr, ExprKind, PrimitiveValue, SolverIdentity, UnaryOperator,
};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

/// Limits for the exact finite-domain reference solver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FiniteModelConfig {
    /// Maximum referenced current/post-state variables in one query.
    pub max_variables: usize,
    /// Maximum values materialized for one integer variable.
    pub max_values_per_integer: usize,
    /// Maximum total assignments explored per query.
    pub max_assignments: usize,
}

impl Default for FiniteModelConfig {
    fn default() -> Self {
        Self {
            max_variables: 256,
            max_values_per_integer: 1_024,
            max_assignments: 1_000_000,
        }
    }
}

/// Exact solver over explicitly bounded integer and boolean declarations.
///
/// It is complete only for the finite domains declared in a contract. Missing
/// bounds or excessive state spaces return `Unknown`, never `Unsat`.
#[derive(Clone, Debug, Default)]
pub struct FiniteModelSolver {
    config: FiniteModelConfig,
}

impl FiniteModelSolver {
    /// Creates the solver with explicit limits.
    #[must_use]
    pub const fn new(config: FiniteModelConfig) -> Self {
        Self { config }
    }
}

impl DecisionProcedure for FiniteModelSolver {
    fn identity(&self) -> SolverIdentity {
        SolverIdentity {
            name: "lambda-safe-finite-model".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            theory: "QF_LIA_FINITE_BOUNDS".to_owned(),
            options: BTreeMap::from([
                (
                    "max_variables".to_owned(),
                    self.config.max_variables.to_string(),
                ),
                (
                    "max_assignments".to_owned(),
                    self.config.max_assignments.to_string(),
                ),
                (
                    "max_values_per_integer".to_owned(),
                    self.config.max_values_per_integer.to_string(),
                ),
            ]),
        }
    }

    fn solve(&self, query: &SolverQuery, timeout: Duration) -> Result<DecisionResult, SolverError> {
        let variables = match build_variables(query, &self.config) {
            Ok(variables) => variables,
            Err(reason) => return Ok(DecisionResult::Unknown(reason)),
        };
        let states = variables.iter().try_fold(1_usize, |product, variable| {
            product.checked_mul(variable.domain.len())
        });
        let Some(states) = states else {
            return Ok(DecisionResult::Unknown(
                "finite-domain state count overflowed".to_owned(),
            ));
        };
        if states > self.config.max_assignments {
            return Ok(DecisionResult::Unknown(format!(
                "finite-domain query requires {states} assignments; limit is {}",
                self.config.max_assignments
            )));
        }

        let Some(deadline) = Instant::now().checked_add(timeout) else {
            return Ok(DecisionResult::Unknown(
                "solver deadline exceeds the platform clock range".to_owned(),
            ));
        };
        let mut assignment = BTreeMap::new();
        let model = search(
            0,
            &variables,
            &mut assignment,
            &query.assertions,
            deadline,
            timeout,
        )?;
        Ok(match model {
            Some(assignments) => DecisionResult::Sat(Counterexample {
                assignments: counterexample_assignments(&assignments, &query.symbols),
                raw_model: None,
            }),
            None => DecisionResult::Unsat,
        })
    }
}

#[derive(Clone, Debug)]
struct Variable {
    key: VariableKey,
    domain: Vec<PrimitiveValue>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct VariableKey {
    name: String,
    state: VariableState,
}

impl VariableKey {
    fn present(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            state: VariableState::Present,
        }
    }

    fn post(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            state: VariableState::Post,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum VariableState {
    Present,
    Post,
}

fn build_variables(
    query: &SolverQuery,
    config: &FiniteModelConfig,
) -> Result<Vec<Variable>, String> {
    let references = referenced_variables(&query.assertions);
    if references.len() > config.max_variables {
        return Err(format!(
            "finite-domain query references {} variables; limit is {}",
            references.len(),
            config.max_variables
        ));
    }
    let symbols = query
        .symbols
        .iter()
        .map(|symbol| (symbol.name.as_str(), symbol))
        .collect::<BTreeMap<_, _>>();
    let mut variables = Vec::new();
    for reference in references {
        let symbol = symbols
            .get(reference.name.as_str())
            .ok_or_else(|| format!("query references undeclared symbol `{}`", reference.name))?;
        let domain = match &symbol.value_type {
            CoreType::Boolean => vec![
                PrimitiveValue::Boolean(false),
                PrimitiveValue::Boolean(true),
            ],
            CoreType::Integer { .. } => {
                let (Some(minimum), Some(maximum)) = (symbol.minimum, symbol.maximum) else {
                    return Err(format!(
                        "finite solver requires lower and upper bounds for integer symbol `{}`",
                        reference.name
                    ));
                };
                if minimum > maximum {
                    return Err(format!("invalid bounds for `{}`", reference.name));
                }
                let size = i128::from(maximum) - i128::from(minimum) + 1;
                if size > config.max_values_per_integer as i128 {
                    return Err(format!(
                        "domain for `{}` has {size} values; per-variable limit is {}",
                        reference.name, config.max_values_per_integer
                    ));
                }
                (minimum..=maximum).map(PrimitiveValue::Integer).collect()
            }
        };
        variables.push(Variable {
            key: reference,
            domain,
        });
    }
    variables.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(variables)
}

fn referenced_variables(assertions: &[Expr]) -> BTreeSet<VariableKey> {
    let mut names = BTreeSet::new();
    for assertion in assertions {
        collect_references(assertion, &mut names);
    }
    names
}

fn collect_references(expression: &Expr, names: &mut BTreeSet<VariableKey>) {
    match &expression.kind {
        ExprKind::Variable { name } => {
            let _ = names.insert(VariableKey::present(name));
        }
        ExprKind::NextVariable { name } => {
            let _ = names.insert(VariableKey::post(name));
        }
        ExprKind::Unary { expression, .. } => collect_references(expression, names),
        ExprKind::Binary { left, right, .. } => {
            collect_references(left, names);
            collect_references(right, names);
        }
        ExprKind::Integer { .. } | ExprKind::Boolean { .. } => {}
    }
}

fn search(
    index: usize,
    variables: &[Variable],
    assignment: &mut BTreeMap<VariableKey, PrimitiveValue>,
    assertions: &[Expr],
    deadline: Instant,
    timeout: Duration,
) -> Result<Option<BTreeMap<VariableKey, PrimitiveValue>>, SolverError> {
    if Instant::now() >= deadline {
        return Err(SolverError::Timeout(timeout.as_millis() as u64));
    }
    if index == variables.len() {
        for assertion in assertions {
            let PrimitiveValue::Boolean(value) = evaluate(assertion, assignment)? else {
                return Err(SolverError::Evaluation(
                    "query assertion did not evaluate to boolean".to_owned(),
                ));
            };
            if !value {
                return Ok(None);
            }
        }
        return Ok(Some(assignment.clone()));
    }

    let variable = &variables[index];
    for value in &variable.domain {
        let _ = assignment.insert(variable.key.clone(), value.clone());
        if let Some(model) = search(
            index + 1,
            variables,
            assignment,
            assertions,
            deadline,
            timeout,
        )? {
            return Ok(Some(model));
        }
    }
    let _ = assignment.remove(&variable.key);
    Ok(None)
}

fn evaluate(
    expression: &Expr,
    assignment: &BTreeMap<VariableKey, PrimitiveValue>,
) -> Result<PrimitiveValue, SolverError> {
    match &expression.kind {
        ExprKind::Integer { value } => Ok(PrimitiveValue::Integer(*value)),
        ExprKind::Boolean { value } => Ok(PrimitiveValue::Boolean(*value)),
        ExprKind::Variable { name } => assignment
            .get(&VariableKey::present(name))
            .cloned()
            .ok_or_else(|| SolverError::Evaluation(format!("missing assignment for `{name}`"))),
        ExprKind::NextVariable { name } => assignment
            .get(&VariableKey::post(name))
            .cloned()
            .ok_or_else(|| {
                SolverError::Evaluation(format!("missing post-state assignment for `{name}`"))
            }),
        ExprKind::Unary {
            operator,
            expression,
        } => match (operator, evaluate(expression, assignment)?) {
            (UnaryOperator::Not, PrimitiveValue::Boolean(value)) => {
                Ok(PrimitiveValue::Boolean(!value))
            }
            (UnaryOperator::Negate, PrimitiveValue::Integer(value)) => value
                .checked_neg()
                .map(PrimitiveValue::Integer)
                .ok_or_else(|| SolverError::Evaluation("integer negation overflow".to_owned())),
            _ => Err(SolverError::Evaluation(
                "invalid operand for unary operator".to_owned(),
            )),
        },
        ExprKind::Binary {
            operator,
            left,
            right,
        } => {
            let left = evaluate(left, assignment)?;
            let right = evaluate(right, assignment)?;
            evaluate_binary(*operator, left, right)
        }
    }
}

fn counterexample_assignments(
    assignment: &BTreeMap<VariableKey, PrimitiveValue>,
    symbols: &[CoreSymbol],
) -> BTreeMap<String, PrimitiveValue> {
    let present_names = symbols
        .iter()
        .map(|symbol| symbol.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut output = BTreeMap::new();
    for (key, value) in assignment {
        let name = match key.state {
            VariableState::Present => key.name.clone(),
            VariableState::Post => {
                let legacy_name = format!("{}__next", key.name);
                if !present_names.contains(legacy_name.as_str()) {
                    legacy_name
                } else {
                    format!("{}@post", key.name)
                }
            }
        };
        let _ = output.insert(name, value.clone());
    }
    output
}

fn evaluate_binary(
    operator: BinaryOperator,
    left: PrimitiveValue,
    right: PrimitiveValue,
) -> Result<PrimitiveValue, SolverError> {
    use BinaryOperator::{
        Add, And, Equal, GreaterThan, GreaterThanOrEqual, Implies, LessThan, LessThanOrEqual,
        Multiply, NotEqual, Or, Subtract,
    };
    match (operator, left, right) {
        (Add, PrimitiveValue::Integer(left), PrimitiveValue::Integer(right)) => left
            .checked_add(right)
            .map(PrimitiveValue::Integer)
            .ok_or_else(|| SolverError::Evaluation("integer addition overflow".to_owned())),
        (Subtract, PrimitiveValue::Integer(left), PrimitiveValue::Integer(right)) => left
            .checked_sub(right)
            .map(PrimitiveValue::Integer)
            .ok_or_else(|| SolverError::Evaluation("integer subtraction overflow".to_owned())),
        (Multiply, PrimitiveValue::Integer(left), PrimitiveValue::Integer(right)) => left
            .checked_mul(right)
            .map(PrimitiveValue::Integer)
            .ok_or_else(|| SolverError::Evaluation("integer multiplication overflow".to_owned())),
        (Equal, left, right) => Ok(PrimitiveValue::Boolean(left == right)),
        (NotEqual, left, right) => Ok(PrimitiveValue::Boolean(left != right)),
        (LessThan, PrimitiveValue::Integer(left), PrimitiveValue::Integer(right)) => {
            Ok(PrimitiveValue::Boolean(left < right))
        }
        (LessThanOrEqual, PrimitiveValue::Integer(left), PrimitiveValue::Integer(right)) => {
            Ok(PrimitiveValue::Boolean(left <= right))
        }
        (GreaterThan, PrimitiveValue::Integer(left), PrimitiveValue::Integer(right)) => {
            Ok(PrimitiveValue::Boolean(left > right))
        }
        (GreaterThanOrEqual, PrimitiveValue::Integer(left), PrimitiveValue::Integer(right)) => {
            Ok(PrimitiveValue::Boolean(left >= right))
        }
        (And, PrimitiveValue::Boolean(left), PrimitiveValue::Boolean(right)) => {
            Ok(PrimitiveValue::Boolean(left && right))
        }
        (Or, PrimitiveValue::Boolean(left), PrimitiveValue::Boolean(right)) => {
            Ok(PrimitiveValue::Boolean(left || right))
        }
        (Implies, PrimitiveValue::Boolean(left), PrimitiveValue::Boolean(right)) => {
            Ok(PrimitiveValue::Boolean(!left || right))
        }
        _ => Err(SolverError::Evaluation(
            "binary operator received values of the wrong sort".to_owned(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_safe_check::{CoreSymbol, CoreType};
    use lambda_safe_model::{AuthorityClass, BinaryOperator, Expr, ExprKind};

    fn int_symbol(name: &str) -> CoreSymbol {
        CoreSymbol {
            name: name.to_owned(),
            value_type: CoreType::Integer {
                base_unit: "count".to_owned(),
                scale: 1,
            },
            authority: AuthorityClass::ControlPlane,
            minimum: Some(0),
            maximum: Some(2),
        }
    }

    fn binary(operator: BinaryOperator, left: Expr, right: Expr) -> Expr {
        Expr::new(ExprKind::Binary {
            operator,
            left: Box::new(left),
            right: Box::new(right),
        })
    }

    #[test]
    fn finds_satisfying_assignment() {
        let query = SolverQuery {
            id: "sat".to_owned(),
            symbols: vec![int_symbol("x")],
            assertions: vec![binary(
                BinaryOperator::GreaterThan,
                Expr::variable("x"),
                Expr::integer(1),
            )],
        };
        let result = FiniteModelSolver::default()
            .solve(&query, Duration::from_secs(1))
            .expect("finite query should execute");
        assert!(matches!(result, DecisionResult::Sat(_)));
    }

    #[test]
    fn proves_bounded_unsatisfiability() {
        let query = SolverQuery {
            id: "unsat".to_owned(),
            symbols: vec![int_symbol("x")],
            assertions: vec![binary(
                BinaryOperator::GreaterThan,
                Expr::variable("x"),
                Expr::integer(2),
            )],
        };
        let result = FiniteModelSolver::default()
            .solve(&query, Duration::from_secs(1))
            .expect("finite query should execute");
        assert_eq!(result, DecisionResult::Unsat);
    }

    #[test]
    fn present_symbol_suffix_does_not_alias_another_symbols_post_state() {
        let mut suffixed_symbol = int_symbol("x__next");
        suffixed_symbol.minimum = Some(0);
        suffixed_symbol.maximum = Some(1);
        let mut base_symbol = int_symbol("x");
        base_symbol.minimum = Some(0);
        base_symbol.maximum = Some(1);
        let query = SolverQuery {
            id: "distinct_present_and_post_state".to_owned(),
            symbols: vec![base_symbol, suffixed_symbol],
            assertions: vec![binary(
                BinaryOperator::NotEqual,
                Expr::variable("x__next"),
                Expr::next_variable("x"),
            )],
        };

        let result = FiniteModelSolver::default()
            .solve(&query, Duration::from_secs(1))
            .expect("finite query should execute");
        let DecisionResult::Sat(counterexample) = result else {
            panic!("present and post-state variables must be independent");
        };
        assert_eq!(
            counterexample.assignments.get("x__next"),
            Some(&PrimitiveValue::Integer(1))
        );
        assert_eq!(
            counterexample.assignments.get("x@post"),
            Some(&PrimitiveValue::Integer(0))
        );
    }
}
