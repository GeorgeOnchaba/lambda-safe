use crate::SyntaxError;
use crate::lexer::{Token, TokenKind, lex};
use lambda_safe_model::CONTRACT_SCHEMA_VERSION;
use lambda_safe_model::{
    AssumptionDefinition, AuthorityClass, BinaryOperator, ClaimClass, ConstantDefinition,
    ContractDocument, Expr, ExprKind, InvariantDefinition, LinearTokenDefinition, SourceSpan,
    SymbolDefinition, TokenConsume, TransitionBranch, TransitionDefinition, TransitionKind,
    UnaryOperator, UnitDefinition, ValueType, codes,
};
use std::collections::BTreeMap;

/// Parses a complete textual λ-Safe contract.
pub fn parse_lambda_source(source: &str) -> Result<ContractDocument, SyntaxError> {
    let tokens = lex(source)?;
    Parser::new(tokens).parse_contract()
}

/// Parses one λ-Safe expression. Used by the JSON projection.
pub fn parse_expression(source: &str) -> Result<Expr, SyntaxError> {
    let tokens = lex(source)?;
    let mut parser = Parser::new(tokens);
    let expression = parser.parse_implication()?;
    parser.expect_kind(&TokenKind::Eof, "end of expression")?;
    Ok(expression)
}

struct Parser {
    tokens: Vec<Token>,
    current: usize,
    nesting_depth: usize,
}

impl Parser {
    const fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
            nesting_depth: 0,
        }
    }

    fn parse_contract(&mut self) -> Result<ContractDocument, SyntaxError> {
        self.expect_word("lambda_safe")?;
        let language_version = self.expect_string("language version string")?;
        self.expect_kind(&TokenKind::Semicolon, "`;`")?;
        self.expect_word("contract")?;
        let contract_id = self.expect_ident("contract identifier")?;
        self.expect_word("version")?;
        let contract_version = self.expect_string("contract version string")?;
        self.expect_kind(&TokenKind::LeftBrace, "`{`")?;

        let mut document = ContractDocument {
            schema_version: CONTRACT_SCHEMA_VERSION.to_owned(),
            language_version,
            contract_id,
            contract_version,
            units: Vec::new(),
            constants: Vec::new(),
            symbols: Vec::new(),
            assumptions: Vec::new(),
            invariants: Vec::new(),
            transitions: Vec::new(),
            tokens: Vec::new(),
            consumes: Vec::new(),
            metadata: BTreeMap::new(),
        };

        while !self.check_kind(&TokenKind::RightBrace) {
            if self.check_kind(&TokenKind::Eof) {
                return Err(self.error_here("expected `}` before end of source"));
            }
            let statement = self.peek_word().ok_or_else(|| {
                self.error_here("expected a declaration such as `unit`, `input` or `invariant`")
            })?;
            match statement {
                "unit" => document.units.push(self.parse_unit()?),
                "const" => document.constants.push(self.parse_constant()?),
                "input" => document.symbols.push(self.parse_input()?),
                "assume" => document.assumptions.push(self.parse_assumption()?),
                "invariant" => document.invariants.push(self.parse_invariant()?),
                "degrade" => document.transitions.push(self.parse_degrade()?),
                "reconcile" => document.transitions.push(self.parse_reconcile()?),
                "token" => document.tokens.push(self.parse_token()?),
                "consume" => document.consumes.push(self.parse_consume()?),
                "metadata" => {
                    let (key, value) = self.parse_metadata()?;
                    if document.metadata.insert(key.clone(), value).is_some() {
                        return Err(self.error_previous(format!("duplicate metadata key `{key}`")));
                    }
                }
                unknown => {
                    return Err(
                        self.error_here(format!("unknown top-level declaration `{unknown}`"))
                    );
                }
            }
        }
        self.expect_kind(&TokenKind::RightBrace, "`}`")?;
        self.expect_kind(&TokenKind::Eof, "end of source")?;
        Ok(document)
    }

    fn parse_unit(&mut self) -> Result<UnitDefinition, SyntaxError> {
        let start = self.expect_word("unit")?;
        let name = self.expect_ident("unit name")?;
        let (scale, base) = if self.match_kind(&TokenKind::Assign) {
            let scale = self.expect_positive_number("unit scale")?;
            let base = self.expect_ident("base unit name")?;
            (scale, base)
        } else {
            (1, name.clone())
        };
        let end = self.expect_kind(&TokenKind::Semicolon, "`;`")?;
        Ok(UnitDefinition {
            name,
            base,
            scale,
            span: start.join(end),
        })
    }

    fn parse_constant(&mut self) -> Result<ConstantDefinition, SyntaxError> {
        let start = self.expect_word("const")?;
        let name = self.expect_ident("constant name")?;
        self.expect_kind(&TokenKind::Colon, "`:`")?;
        let unit = self.parse_integer_type_unit()?;
        self.expect_kind(&TokenKind::Assign, "`=`")?;
        let value = self.parse_signed_number("constant value")?;
        let end = self.expect_kind(&TokenKind::Semicolon, "`;`")?;
        Ok(ConstantDefinition {
            name,
            value,
            unit,
            span: start.join(end),
        })
    }

    fn parse_input(&mut self) -> Result<SymbolDefinition, SyntaxError> {
        let start = self.expect_word("input")?;
        let name = self.expect_ident("input name")?;
        self.expect_kind(&TokenKind::Colon, "`:`")?;
        let value_type = if self.match_word("bool") {
            ValueType::Boolean
        } else {
            ValueType::integer(self.parse_integer_type_unit()?)
        };
        self.expect_word("authority")?;
        let authority_name = self.expect_ident("authority class")?;
        let authority = parse_authority(&authority_name).ok_or_else(|| {
            self.error_previous(format!("unknown authority class `{authority_name}`"))
        })?;

        let (minimum, maximum) = if self.match_word("range") {
            self.expect_kind(&TokenKind::LeftBracket, "`[`")?;
            let minimum = self.parse_signed_number("range minimum")?;
            self.expect_kind(&TokenKind::Comma, "`,`")?;
            let maximum = self.parse_signed_number("range maximum")?;
            self.expect_kind(&TokenKind::RightBracket, "`]`")?;
            (Some(minimum), Some(maximum))
        } else {
            (None, None)
        };

        let end = self.expect_kind(&TokenKind::Semicolon, "`;`")?;
        Ok(SymbolDefinition {
            name,
            value_type,
            authority,
            minimum,
            maximum,
            provenance: None,
            span: start.join(end),
        })
    }

    fn parse_assumption(&mut self) -> Result<AssumptionDefinition, SyntaxError> {
        let start = self.expect_word("assume")?;
        let id = self.expect_ident("assumption identifier")?;
        let class = self.parse_claim_class()?;
        self.expect_kind(&TokenKind::Colon, "`:`")?;
        let expression = self.parse_implication()?;
        let end = self.expect_kind(&TokenKind::Semicolon, "`;`")?;
        Ok(AssumptionDefinition {
            id,
            class,
            expression,
            span: start.join(end),
        })
    }

    fn parse_invariant(&mut self) -> Result<InvariantDefinition, SyntaxError> {
        let start = self.expect_word("invariant")?;
        let id = self.expect_ident("invariant identifier")?;
        let class = self.parse_claim_class()?;
        self.expect_kind(&TokenKind::Colon, "`:`")?;
        let expression = self.parse_implication()?;
        let end = self.expect_kind(&TokenKind::Semicolon, "`;`")?;
        Ok(InvariantDefinition {
            id,
            class,
            expression,
            span: start.join(end),
        })
    }

    fn parse_degrade(&mut self) -> Result<TransitionDefinition, SyntaxError> {
        let start = self.expect_word("degrade")?;
        let id = self.expect_ident("degrade identifier")?;
        self.expect_word("preserves")?;
        let preserves = self.parse_identifier_list()?;
        self.expect_kind(&TokenKind::LeftBrace, "`{`")?;
        let primary = self.parse_named_relation("primary")?;
        let fallback = self.parse_named_relation("fallback")?;
        let end = self.expect_kind(&TokenKind::RightBrace, "`}`")?;
        Ok(TransitionDefinition {
            id,
            kind: TransitionKind::Degrade,
            strategy: None,
            preserves,
            branches: vec![primary, fallback],
            span: start.join(end),
        })
    }

    fn parse_reconcile(&mut self) -> Result<TransitionDefinition, SyntaxError> {
        let start = self.expect_word("reconcile")?;
        let id = self.expect_ident("reconcile identifier")?;
        self.expect_word("using")?;
        let strategy = self.expect_ident("strategy identifier")?;
        self.expect_word("preserves")?;
        let preserves = self.parse_identifier_list()?;
        self.expect_kind(&TokenKind::LeftBrace, "`{`")?;
        let merge = self.parse_named_relation("merge")?;
        let end = self.expect_kind(&TokenKind::RightBrace, "`}`")?;
        Ok(TransitionDefinition {
            id,
            kind: TransitionKind::Reconcile,
            strategy: Some(strategy),
            preserves,
            branches: vec![merge],
            span: start.join(end),
        })
    }

    fn parse_named_relation(
        &mut self,
        expected_name: &str,
    ) -> Result<TransitionBranch, SyntaxError> {
        let start = self.expect_word(expected_name)?;
        self.expect_kind(&TokenKind::Colon, "`:`")?;
        let relation = self.parse_implication()?;
        let end = self.expect_kind(&TokenKind::Semicolon, "`;`")?;
        Ok(TransitionBranch {
            name: expected_name.to_owned(),
            relation,
            span: start.join(end),
        })
    }

    fn parse_token(&mut self) -> Result<LinearTokenDefinition, SyntaxError> {
        let start = self.expect_word("token")?;
        let name = self.expect_ident("token name")?;
        let end = self.expect_kind(&TokenKind::Semicolon, "`;`")?;
        Ok(LinearTokenDefinition {
            name,
            span: start.join(end),
        })
    }

    fn parse_consume(&mut self) -> Result<TokenConsume, SyntaxError> {
        let start = self.expect_word("consume")?;
        let name = self.expect_ident("token name")?;
        let end = self.expect_kind(&TokenKind::Semicolon, "`;`")?;
        Ok(TokenConsume {
            name,
            span: start.join(end),
        })
    }

    fn parse_metadata(&mut self) -> Result<(String, String), SyntaxError> {
        self.expect_word("metadata")?;
        let key = self.expect_ident("metadata key")?;
        self.expect_kind(&TokenKind::Assign, "`=`")?;
        let value = self.expect_string("metadata string")?;
        self.expect_kind(&TokenKind::Semicolon, "`;`")?;
        Ok((key, value))
    }

    fn parse_identifier_list(&mut self) -> Result<Vec<String>, SyntaxError> {
        self.expect_kind(&TokenKind::LeftBracket, "`[`")?;
        let mut identifiers = Vec::new();
        if !self.check_kind(&TokenKind::RightBracket) {
            loop {
                identifiers.push(self.expect_ident("identifier")?);
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect_kind(&TokenKind::RightBracket, "`]`")?;
        Ok(identifiers)
    }

    fn parse_integer_type_unit(&mut self) -> Result<String, SyntaxError> {
        self.expect_word("int")?;
        self.expect_kind(&TokenKind::LeftAngle, "`<`")?;
        let unit = self.expect_ident("unit name")?;
        self.expect_kind(&TokenKind::RightAngle, "`>`")?;
        Ok(unit)
    }

    fn parse_claim_class(&mut self) -> Result<ClaimClass, SyntaxError> {
        let name = self.expect_ident("claim class")?;
        match name.as_str() {
            "hard" => Ok(ClaimClass::Hard),
            "trusted" => Ok(ClaimClass::Trusted),
            "soft" => Ok(ClaimClass::Soft),
            _ => Err(self.error_previous(format!("unknown claim class `{name}`"))),
        }
    }

    fn parse_implication(&mut self) -> Result<Expr, SyntaxError> {
        let left = self.parse_or()?;
        if self.match_kind(&TokenKind::Implies) {
            let span = self.previous().span;
            self.enter_nesting(span)?;
            let right = self.parse_implication();
            self.leave_nesting();
            let right = right?;
            Ok(binary(BinaryOperator::Implies, left, right))
        } else {
            Ok(left)
        }
    }

    fn parse_or(&mut self) -> Result<Expr, SyntaxError> {
        let mut expression = self.parse_and()?;
        while self.match_kind(&TokenKind::OrOr) {
            let right = self.parse_and()?;
            expression = binary(BinaryOperator::Or, expression, right);
        }
        Ok(expression)
    }

    fn parse_and(&mut self) -> Result<Expr, SyntaxError> {
        let mut expression = self.parse_equality()?;
        while self.match_kind(&TokenKind::AndAnd) {
            let right = self.parse_equality()?;
            expression = binary(BinaryOperator::And, expression, right);
        }
        Ok(expression)
    }

    fn parse_equality(&mut self) -> Result<Expr, SyntaxError> {
        let mut expression = self.parse_comparison()?;
        loop {
            let operator = if self.match_kind(&TokenKind::EqualEqual) {
                Some(BinaryOperator::Equal)
            } else if self.match_kind(&TokenKind::BangEqual) {
                Some(BinaryOperator::NotEqual)
            } else {
                None
            };
            let Some(operator) = operator else { break };
            let right = self.parse_comparison()?;
            expression = binary(operator, expression, right);
        }
        Ok(expression)
    }

    fn parse_comparison(&mut self) -> Result<Expr, SyntaxError> {
        let mut expression = self.parse_term()?;
        loop {
            let operator = if self.match_kind(&TokenKind::LeftAngle) {
                Some(BinaryOperator::LessThan)
            } else if self.match_kind(&TokenKind::LessEqual) {
                Some(BinaryOperator::LessThanOrEqual)
            } else if self.match_kind(&TokenKind::RightAngle) {
                Some(BinaryOperator::GreaterThan)
            } else if self.match_kind(&TokenKind::GreaterEqual) {
                Some(BinaryOperator::GreaterThanOrEqual)
            } else {
                None
            };
            let Some(operator) = operator else { break };
            let right = self.parse_term()?;
            expression = binary(operator, expression, right);
        }
        Ok(expression)
    }

    fn parse_term(&mut self) -> Result<Expr, SyntaxError> {
        let mut expression = self.parse_factor()?;
        loop {
            let operator = if self.match_kind(&TokenKind::Plus) {
                Some(BinaryOperator::Add)
            } else if self.match_kind(&TokenKind::Minus) {
                Some(BinaryOperator::Subtract)
            } else {
                None
            };
            let Some(operator) = operator else { break };
            let right = self.parse_factor()?;
            expression = binary(operator, expression, right);
        }
        Ok(expression)
    }

    fn parse_factor(&mut self) -> Result<Expr, SyntaxError> {
        let mut expression = self.parse_unary()?;
        while self.match_kind(&TokenKind::Star) {
            let right = self.parse_unary()?;
            expression = binary(BinaryOperator::Multiply, expression, right);
        }
        Ok(expression)
    }

    fn parse_unary(&mut self) -> Result<Expr, SyntaxError> {
        if self.match_kind(&TokenKind::Bang) {
            let operator_span = self.previous().span;
            self.enter_nesting(operator_span)?;
            let expression = self.parse_unary();
            self.leave_nesting();
            let expression = expression?;
            let span = operator_span.join(expression.span);
            return Ok(Expr {
                kind: ExprKind::Unary {
                    operator: UnaryOperator::Not,
                    expression: Box::new(expression),
                },
                span,
            });
        }
        if self.match_kind(&TokenKind::Minus) {
            let operator_span = self.previous().span;
            self.enter_nesting(operator_span)?;
            let expression = self.parse_unary();
            self.leave_nesting();
            let expression = expression?;
            let span = operator_span.join(expression.span);
            return Ok(Expr {
                kind: ExprKind::Unary {
                    operator: UnaryOperator::Negate,
                    expression: Box::new(expression),
                },
                span,
            });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, SyntaxError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Number(value) => Ok(Expr {
                kind: ExprKind::Integer { value },
                span: token.span,
            }),
            TokenKind::Ident(name) if name == "true" => Ok(Expr {
                kind: ExprKind::Boolean { value: true },
                span: token.span,
            }),
            TokenKind::Ident(name) if name == "false" => Ok(Expr {
                kind: ExprKind::Boolean { value: false },
                span: token.span,
            }),
            TokenKind::Ident(name) if name == "next" => {
                self.expect_kind(&TokenKind::LeftParen, "`(` after `next`")?;
                let variable = self.expect_ident("state variable inside `next(...)`")?;
                let end = self.expect_kind(&TokenKind::RightParen, "`)`")?;
                Ok(Expr {
                    kind: ExprKind::NextVariable { name: variable },
                    span: token.span.join(end),
                })
            }
            TokenKind::Ident(name) => Ok(Expr {
                kind: ExprKind::Variable { name },
                span: token.span,
            }),
            TokenKind::LeftParen => {
                self.enter_nesting(token.span)?;
                let expression = self.parse_implication();
                self.leave_nesting();
                let expression = expression?;
                let end = self.expect_kind(&TokenKind::RightParen, "`)`")?;
                Ok(Expr {
                    span: token.span.join(end),
                    ..expression
                })
            }
            _ => Err(SyntaxError::new(
                lambda_safe_model::Diagnostic::error(
                    codes::INVALID_SOURCE,
                    "expected an integer, boolean, variable, `next(name)` or parenthesized expression",
                )
                .at(token.span),
            )),
        }
    }

    fn expect_positive_number(&mut self, expected: &str) -> Result<i64, SyntaxError> {
        let value = self.parse_signed_number(expected)?;
        if value <= 0 {
            return Err(self.error_previous(format!("{expected} must be greater than zero")));
        }
        Ok(value)
    }

    fn parse_signed_number(&mut self, expected: &str) -> Result<i64, SyntaxError> {
        let negative = self.match_kind(&TokenKind::Minus);
        let token = self.advance().clone();
        let TokenKind::Number(value) = token.kind else {
            return Err(SyntaxError::new(
                lambda_safe_model::Diagnostic::error(
                    codes::INVALID_SOURCE,
                    format!("expected {expected}"),
                )
                .at(token.span),
            ));
        };
        if negative {
            value
                .checked_neg()
                .ok_or_else(|| self.error_previous(format!("{expected} is outside the i64 range")))
        } else {
            Ok(value)
        }
    }

    fn expect_word(&mut self, expected: &str) -> Result<SourceSpan, SyntaxError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Ident(actual) if actual == expected => Ok(token.span),
            _ => Err(SyntaxError::new(
                lambda_safe_model::Diagnostic::error(
                    codes::INVALID_SOURCE,
                    format!("expected keyword `{expected}`"),
                )
                .at(token.span),
            )),
        }
    }

    fn expect_ident(&mut self, expected: &str) -> Result<String, SyntaxError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Ident(value) => Ok(value),
            _ => Err(SyntaxError::new(
                lambda_safe_model::Diagnostic::error(
                    codes::INVALID_SOURCE,
                    format!("expected {expected}"),
                )
                .at(token.span),
            )),
        }
    }

    fn expect_string(&mut self, expected: &str) -> Result<String, SyntaxError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::String(value) => Ok(value),
            _ => Err(SyntaxError::new(
                lambda_safe_model::Diagnostic::error(
                    codes::INVALID_SOURCE,
                    format!("expected {expected}"),
                )
                .at(token.span),
            )),
        }
    }

    fn expect_kind(
        &mut self,
        expected: &TokenKind,
        description: &str,
    ) -> Result<SourceSpan, SyntaxError> {
        let token = self.advance().clone();
        if same_kind(&token.kind, expected) {
            Ok(token.span)
        } else {
            Err(SyntaxError::new(
                lambda_safe_model::Diagnostic::error(
                    codes::INVALID_SOURCE,
                    format!("expected {description}"),
                )
                .at(token.span),
            ))
        }
    }

    fn match_word(&mut self, expected: &str) -> bool {
        if self.peek_word() == Some(expected) {
            self.current += 1;
            true
        } else {
            false
        }
    }

    fn match_kind(&mut self, expected: &TokenKind) -> bool {
        if self.check_kind(expected) {
            self.current += 1;
            true
        } else {
            false
        }
    }

    fn check_kind(&self, expected: &TokenKind) -> bool {
        same_kind(&self.peek().kind, expected)
    }

    fn peek_word(&self) -> Option<&str> {
        match &self.peek().kind {
            TokenKind::Ident(value) => Some(value.as_str()),
            _ => None,
        }
    }

    fn advance(&mut self) -> &Token {
        let index = self.current;
        if !self.check_kind(&TokenKind::Eof) {
            self.current += 1;
        }
        &self.tokens[index]
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current.saturating_sub(1)]
    }

    fn error_here(&self, message: impl Into<String>) -> SyntaxError {
        SyntaxError::new(
            lambda_safe_model::Diagnostic::error(codes::INVALID_SOURCE, message)
                .at(self.peek().span),
        )
    }

    fn error_previous(&self, message: impl Into<String>) -> SyntaxError {
        SyntaxError::new(
            lambda_safe_model::Diagnostic::error(codes::INVALID_SOURCE, message)
                .at(self.previous().span),
        )
    }

    fn enter_nesting(&mut self, span: SourceSpan) -> Result<(), SyntaxError> {
        const MAX_EXPRESSION_NESTING: usize = 256;
        if self.nesting_depth >= MAX_EXPRESSION_NESTING {
            return Err(SyntaxError::new(
                lambda_safe_model::Diagnostic::error(
                    codes::RESOURCE_LIMIT,
                    format!(
                        "expression nesting exceeds the supported depth of {MAX_EXPRESSION_NESTING}"
                    ),
                )
                .at(span),
            ));
        }
        self.nesting_depth += 1;
        Ok(())
    }

    fn leave_nesting(&mut self) {
        self.nesting_depth = self.nesting_depth.saturating_sub(1);
    }
}

fn parse_authority(name: &str) -> Option<AuthorityClass> {
    match name {
        "control_plane" => Some(AuthorityClass::ControlPlane),
        "constant" => Some(AuthorityClass::Constant),
        "trusted_report" => Some(AuthorityClass::TrustedReport),
        "predictive" => Some(AuthorityClass::Predictive),
        "unsupported" => Some(AuthorityClass::Unsupported),
        _ => None,
    }
}

fn binary(operator: BinaryOperator, left: Expr, right: Expr) -> Expr {
    let span = left.span.join(right.span);
    Expr {
        kind: ExprKind::Binary {
            operator,
            left: Box::new(left),
            right: Box::new(right),
        },
        span,
    }
}

fn same_kind(left: &TokenKind, right: &TokenKind) -> bool {
    std::mem::discriminant(left) == std::mem::discriminant(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_expression_precedence() {
        let expression = parse_expression("a + 2 * b <= c && enabled");
        assert!(expression.is_ok());
    }

    #[test]
    fn parses_minimal_contract() {
        let source = r#"
            lambda_safe "0.1";
            contract capacity version "1.0.0" {
                unit cpu_milli;
                const zero: int<cpu_milli> = 0;
                input allocated: int<cpu_milli> authority control_plane range [0, 100];
                input capacity: int<cpu_milli> authority control_plane range [0, 100];
                assume nonnegative hard: capacity >= zero;
                invariant bounded hard: allocated <= capacity;
            }
        "#;
        let document = parse_lambda_source(source).expect("valid contract should parse");
        assert_eq!(
            document.language_version,
            lambda_safe_model::LANGUAGE_VERSION
        );
        assert_eq!(document.invariants.len(), 1);
    }

    #[test]
    fn rejects_excessive_expression_nesting() {
        let source = format!("{}true{}", "(".repeat(300), ")".repeat(300));
        assert!(parse_expression(&source).is_err());
    }
}

