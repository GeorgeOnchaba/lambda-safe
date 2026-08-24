use crate::{SyntaxError, deserialize_json_strict, parse_expression};
use lambda_safe_model::{
    AssumptionDefinition, ConstantDefinition, ContractDocument, ContractProjection, Diagnostic,
    InvariantDefinition, LinearTokenDefinition, SourceSpan, SymbolDefinition, TokenConsume,
    TransitionBranch, TransitionDefinition, UnitDefinition, codes,
};

/// Parses the operational JSON projection and lowers every expression string
/// through the same expression parser used by textual contracts.
pub fn parse_json_projection(source: &str) -> Result<ContractDocument, SyntaxError> {
    let projection: ContractProjection = deserialize_json_strict(source).map_err(|error| {
        SyntaxError::new(
            Diagnostic::error(
                codes::INVALID_SOURCE,
                format!("invalid contract JSON projection: {error}"),
            )
            .with_help("Validate the document against schemas/contract.schema.json."),
        )
    })?;

    let assumptions = projection
        .assumptions
        .into_iter()
        .map(|assumption| {
            Ok(AssumptionDefinition {
                id: assumption.id,
                class: assumption.class,
                expression: parse_expression(&assumption.expression)?,
                span: SourceSpan::unknown(),
            })
        })
        .collect::<Result<Vec<_>, SyntaxError>>()?;

    let invariants = projection
        .invariants
        .into_iter()
        .map(|invariant| {
            Ok(InvariantDefinition {
                id: invariant.id,
                class: invariant.class,
                expression: parse_expression(&invariant.expression)?,
                span: SourceSpan::unknown(),
            })
        })
        .collect::<Result<Vec<_>, SyntaxError>>()?;

    let transitions = projection
        .transitions
        .into_iter()
        .map(|transition| {
            let branches = transition
                .branches
                .into_iter()
                .map(|branch| {
                    Ok(TransitionBranch {
                        name: branch.name,
                        relation: parse_expression(&branch.relation)?,
                        span: SourceSpan::unknown(),
                    })
                })
                .collect::<Result<Vec<_>, SyntaxError>>()?;
            Ok(TransitionDefinition {
                id: transition.id,
                kind: transition.kind,
                strategy: transition.strategy,
                preserves: transition.preserves,
                branches,
                span: SourceSpan::unknown(),
            })
        })
        .collect::<Result<Vec<_>, SyntaxError>>()?;

    let units = projection
        .units
        .into_iter()
        .map(|unit| UnitDefinition {
            name: unit.name,
            base: unit.base,
            scale: unit.scale,
            span: SourceSpan::unknown(),
        })
        .collect();
    let constants = projection
        .constants
        .into_iter()
        .map(|constant| ConstantDefinition {
            name: constant.name,
            value: constant.value,
            unit: constant.unit,
            span: SourceSpan::unknown(),
        })
        .collect();
    let symbols = projection
        .symbols
        .into_iter()
        .map(|symbol| SymbolDefinition {
            name: symbol.name,
            value_type: symbol.value_type,
            authority: symbol.authority,
            minimum: symbol.minimum,
            maximum: symbol.maximum,
            provenance: symbol.provenance,
            span: SourceSpan::unknown(),
        })
        .collect();
    let tokens = projection
        .tokens
        .into_iter()
        .map(|token| LinearTokenDefinition {
            name: token.name,
            span: SourceSpan::unknown(),
        })
        .collect();
    let consumes = projection
        .consumes
        .into_iter()
        .map(|consume| TokenConsume {
            name: consume.name,
            span: SourceSpan::unknown(),
        })
        .collect();

    Ok(ContractDocument {
        schema_version: projection.schema_version,
        language_version: projection.language_version,
        contract_id: projection.contract_id,
        contract_version: projection.contract_version,
        units,
        constants,
        symbols,
        assumptions,
        invariants,
        transitions,
        tokens,
        consumes,
        metadata: projection.metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_json_fields() {
        let source = r#"{
          "schema_version":"0.1",
          "language_version":"0.1",
          "contract_id":"x",
          "contract_version":"1",
          "unexpected":true
        }"#;
        assert!(parse_json_projection(source).is_err());
    }

    #[test]
    fn rejects_unknown_nested_json_fields() {
        let source = r#"{
          "schema_version":"0.1",
          "language_version":"0.1",
          "contract_id":"x",
          "contract_version":"1",
          "units":[{"name":"count","base":"count","scale":1,"span":{}}]
        }"#;
        assert!(parse_json_projection(source).is_err());
    }

    #[test]
    fn rejects_duplicate_json_fields_at_every_depth() {
        let root_duplicate = r#"{
          "schema_version":"0.1",
          "schema_version":"0.1",
          "language_version":"0.1",
          "contract_id":"x",
          "contract_version":"1"
        }"#;
        assert!(parse_json_projection(root_duplicate).is_err());

        let nested_duplicate = r#"{
          "schema_version":"0.1",
          "language_version":"0.1",
          "contract_id":"x",
          "contract_version":"1",
          "units":[{"name":"count","name":"other","base":"count","scale":1}]
        }"#;
        assert!(parse_json_projection(nested_duplicate).is_err());
    }
}
