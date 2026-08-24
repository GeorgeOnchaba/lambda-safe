use crate::query::SolverQuery;
use lambda_safe_check::CoreType;
use lambda_safe_model::{BinaryOperator, Expr, ExprKind, UnaryOperator};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::time::Duration;

/// Emits a deterministic SMT-LIB 2 script for Z3 or another compatible solver.
#[must_use]
pub fn emit_smt2(query: &SolverQuery, timeout: Duration) -> String {
    let mut output = String::new();
    output.push_str("(set-logic QF_LIA)\n");
    output.push_str("(set-option :auto-config false)\n");
    output.push_str("(set-option :produce-models true)\n");
    output.push_str(&format!("(set-option :timeout {})\n", timeout.as_millis()));

    let next_names = referenced_next_names(&query.assertions);
    for symbol in &query.symbols {
        let sort = match &symbol.value_type {
            CoreType::Boolean => "Bool",
            CoreType::Integer { .. } => "Int",
        };
        output.push_str(&format!(
            "(declare-fun {} () {sort})\n",
            smt_name(&symbol.name, false)
        ));
        if next_names.contains(&symbol.name) {
            output.push_str(&format!(
                "(declare-fun {} () {sort})\n",
                smt_name(&symbol.name, true)
            ));
        }
    }

    for symbol in &query.symbols {
        if matches!(&symbol.value_type, CoreType::Integer { .. }) {
            if let Some(minimum) = symbol.minimum {
                output.push_str(&format!(
                    "(assert (>= {} {minimum}))\n",
                    smt_name(&symbol.name, false)
                ));
                if next_names.contains(&symbol.name) {
                    output.push_str(&format!(
                        "(assert (>= {} {minimum}))\n",
                        smt_name(&symbol.name, true)
                    ));
                }
            }
            if let Some(maximum) = symbol.maximum {
                output.push_str(&format!(
                    "(assert (<= {} {maximum}))\n",
                    smt_name(&symbol.name, false)
                ));
                if next_names.contains(&symbol.name) {
                    output.push_str(&format!(
                        "(assert (<= {} {maximum}))\n",
                        smt_name(&symbol.name, true)
                    ));
                }
            }
        }
    }

    for assertion in &query.assertions {
        output.push_str("(assert ");
        output.push_str(&emit_expression(assertion));
        output.push_str(")\n");
    }
    output.push_str("(check-sat)\n");
    output
}

fn referenced_next_names(assertions: &[Expr]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for expression in assertions {
        collect_next_names(expression, &mut names);
    }
    names
}

fn collect_next_names(expression: &Expr, names: &mut BTreeSet<String>) {
    match &expression.kind {
        ExprKind::NextVariable { name } => {
            let _ = names.insert(name.clone());
        }
        ExprKind::Unary { expression, .. } => collect_next_names(expression, names),
        ExprKind::Binary { left, right, .. } => {
            collect_next_names(left, names);
            collect_next_names(right, names);
        }
        ExprKind::Integer { .. } | ExprKind::Boolean { .. } | ExprKind::Variable { .. } => {}
    }
}

fn emit_expression(expression: &Expr) -> String {
    match &expression.kind {
        ExprKind::Integer { value } if *value < 0 => format!("(- {})", value.unsigned_abs()),
        ExprKind::Integer { value } => value.to_string(),
        ExprKind::Boolean { value } => value.to_string(),
        ExprKind::Variable { name } => smt_name(name, false),
        ExprKind::NextVariable { name } => smt_name(name, true),
        ExprKind::Unary {
            operator,
            expression,
        } => match operator {
            UnaryOperator::Not => format!("(not {})", emit_expression(expression)),
            UnaryOperator::Negate => format!("(- {})", emit_expression(expression)),
        },
        ExprKind::Binary {
            operator,
            left,
            right,
        } => {
            let left = emit_expression(left);
            let right = emit_expression(right);
            match operator {
                BinaryOperator::NotEqual => format!("(not (= {left} {right}))"),
                other => format!("({} {left} {right})", binary_symbol(*other)),
            }
        }
    }
}

fn smt_name(name: &str, next: bool) -> String {
    let mut encoded = String::with_capacity(name.len() * 2);
    for byte in name.as_bytes() {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    let state = if next { "next" } else { "pre" };
    format!("|ls_{encoded}_{state}|")
}

const fn binary_symbol(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Add => "+",
        BinaryOperator::Subtract => "-",
        BinaryOperator::Multiply => "*",
        BinaryOperator::Equal => "=",
        BinaryOperator::NotEqual => "distinct",
        BinaryOperator::LessThan => "<",
        BinaryOperator::LessThanOrEqual => "<=",
        BinaryOperator::GreaterThan => ">",
        BinaryOperator::GreaterThanOrEqual => ">=",
        BinaryOperator::And => "and",
        BinaryOperator::Or => "or",
        BinaryOperator::Implies => "=>",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_safe_check::{CoreSymbol, CoreType};
    use lambda_safe_model::{AuthorityClass, Expr};

    #[test]
    fn emits_logic_declarations_and_bounds() {
        let query = SolverQuery {
            id: "q".to_owned(),
            symbols: vec![CoreSymbol {
                name: "x".to_owned(),
                value_type: CoreType::Integer {
                    base_unit: "count".to_owned(),
                    scale: 1,
                },
                authority: AuthorityClass::ControlPlane,
                minimum: Some(0),
                maximum: Some(10),
            }],
            assertions: vec![Expr::boolean(true)],
        };
        let script = emit_smt2(&query, Duration::from_millis(100));
        assert!(script.contains("(set-logic QF_LIA)"));
        assert!(script.contains("(assert (>= |ls_78_pre| 0))"));
        assert!(script.contains("(check-sat)"));
    }
}
