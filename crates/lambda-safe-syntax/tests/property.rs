use lambda_safe_syntax::parse_expression;
use proptest::prelude::*;

proptest! {
    #[test]
    fn integer_reflexivity_expression_always_parses(name in "[a-z][a-z0-9_]{0,20}") {
        let source = format!("{name} <= {name}");
        prop_assert!(parse_expression(&source).is_ok());
    }

    #[test]
    fn comments_after_expression_are_ignored(value in 0_i64..1_000_000) {
        let source = format!("{value} == {value} // generated");
        prop_assert!(parse_expression(&source).is_ok());
    }
}
