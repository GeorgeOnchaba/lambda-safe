use lambda_safe_check::{CoreSymbol, CoreType};
use lambda_safe_model::{AuthorityClass, BinaryOperator, Expr, ExprKind};
use lambda_safe_solver::{
    DecisionProcedure, DecisionResult, FiniteModelSolver, SolverError, SolverQuery, Z3ProcessSolver,
};
use std::io::ErrorKind;
use std::process::Command;
use std::time::Duration;

fn symbol(name: &str, minimum: i64, maximum: i64) -> CoreSymbol {
    CoreSymbol {
        name: name.to_owned(),
        value_type: CoreType::Integer {
            base_unit: "count".to_owned(),
            scale: 1,
        },
        authority: AuthorityClass::ControlPlane,
        minimum: Some(minimum),
        maximum: Some(maximum),
    }
}

fn binary(operator: BinaryOperator, left: Expr, right: Expr) -> Expr {
    Expr::new(ExprKind::Binary {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn is_sat(result: Result<DecisionResult, SolverError>, query_id: &str) -> bool {
    match result.unwrap_or_else(|error| panic!("{query_id}: solver failed: {error}")) {
        DecisionResult::Sat(_) => true,
        DecisionResult::Unsat => false,
        DecisionResult::Unknown(reason) => panic!("{query_id}: solver returned unknown: {reason}"),
    }
}

#[test]
fn bounded_qf_lia_agrees_with_z3_when_z3_is_installed() {
    match Command::new("z3").arg("-version").output() {
        Err(error) if error.kind() == ErrorKind::NotFound => {
            eprintln!("skipping finite/Z3 differential corpus: z3 is not installed");
            return;
        }
        Err(error) => panic!("cannot execute installed z3: {error}"),
        Ok(output) if !output.status.success() => {
            panic!("z3 -version exited with {}", output.status)
        }
        Ok(_) => {}
    }

    let z3 = Z3ProcessSolver::discover("z3").expect("installed Z3 should report its version");
    let finite = FiniteModelSolver::default();
    let symbols = vec![symbol("x", -2, 2), symbol("y", -2, 2)];
    let mut queries = Vec::new();

    for threshold in -4..=4 {
        let sum = || {
            binary(
                BinaryOperator::Add,
                Expr::variable("x"),
                Expr::variable("y"),
            )
        };
        for operator in [BinaryOperator::LessThanOrEqual, BinaryOperator::Equal] {
            queries.push(SolverQuery {
                id: format!("sum_{operator:?}_{threshold}"),
                symbols: symbols.clone(),
                assertions: vec![binary(operator, sum(), Expr::integer(threshold))],
            });
        }
    }

    queries.push(SolverQuery {
        id: "present_name_does_not_alias_post_state".to_owned(),
        symbols: vec![symbol("x", 0, 1), symbol("x__next", 0, 1)],
        assertions: vec![binary(
            BinaryOperator::NotEqual,
            Expr::next_variable("x"),
            Expr::variable("x__next"),
        )],
    });

    for query in queries {
        let finite_result = finite.solve(&query, Duration::from_secs(2));
        let z3_result = z3.solve(&query, Duration::from_secs(2));
        assert_eq!(
            is_sat(finite_result, &query.id),
            is_sat(z3_result, &query.id),
            "finite solver and Z3 disagree for {}",
            query.id
        );
    }
}
