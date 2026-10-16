// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use cel::{Context, Program, Value};
use serde::{Deserialize, Serialize};

use crate::{errors::ModelError, types::context::InvocationContext};

const CONTEXT_VARIABLE: &str = "ctx";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CelExpression {
    program: Arc<Program>,
    source: String,
}

impl CelExpression {
    pub fn new(source: impl Into<String>) -> Result<Self, ModelError> {
        let source = source.into();

        Ok(Self {
            program: Arc::new(Program::compile(&source).map_err(|e| {
                ModelError::configuration(format!("header expression '{source}': {e}"))
            })?),
            source,
        })
    }

    pub fn evaluate(&self, context: &InvocationContext) -> Result<String, ModelError> {
        let mut variables = Context::default();

        variables
            .add_variable(CONTEXT_VARIABLE, context)
            .map_err(|e| ModelError::invalid_request(e.to_string()))?;

        match self
            .program
            .execute(&variables)
            .map_err(|e| ModelError::invalid_request(e.to_string()))?
        {
            Value::String(value) => Ok(value.to_string()),
            other => Err(ModelError::invalid_request(format!(
                "header expression evaluated to {}, expected a string",
                other.type_of()
            ))),
        }
    }
}

impl TryFrom<String> for CelExpression {
    type Error = ModelError;

    fn try_from(source: String) -> Result<Self, ModelError> {
        Self::new(source)
    }
}

impl From<CelExpression> for String {
    fn from(expression: CelExpression) -> Self {
        expression.source
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::providers::rig::headers::HeaderSource;

    use super::*;

    #[test]
    fn evaluates_attributes() {
        assert_eq!(
            CelExpression::new("'run-' + ctx.attributes.run_id")
                .unwrap()
                .evaluate(&InvocationContext::new().with_attribute("run_id", "r1"))
                .unwrap(),
            "run-r1"
        );
    }

    #[test]
    fn evaluates_typed_fields() {
        assert_eq!(
            CelExpression::new("'tenant-' + ctx.tenant_id")
                .unwrap()
                .evaluate(&InvocationContext::new().with_tenant_id("acme"))
                .unwrap(),
            "tenant-acme"
        );
    }

    #[test]
    fn rejects_absent_value() {
        assert!(
            CelExpression::new("ctx.attributes.run_id")
                .unwrap()
                .evaluate(&InvocationContext::new())
                .is_err()
        );
    }

    #[test]
    fn rejects_absent_typed_value() {
        assert!(
            CelExpression::new("ctx.tenant_id")
                .unwrap()
                .evaluate(&InvocationContext::new())
                .is_err()
        );
    }

    #[test]
    fn rejects_non_string_result() {
        assert!(
            CelExpression::new("1 + 1")
                .unwrap()
                .evaluate(&InvocationContext::new())
                .is_err()
        );
    }

    #[test]
    fn rejects_malformed_source() {
        assert!(CelExpression::new("ctx.tenant_id +").is_err());
    }

    #[test]
    fn round_trips_through_header_source() {
        let value = serde_json::to_value(HeaderSource::cel("ctx.tenant_id").unwrap()).unwrap();

        assert_eq!(value, json!({ "syntax": "cel", "source": "ctx.tenant_id" }));
        assert!(matches!(
            serde_json::from_value::<HeaderSource>(value).unwrap(),
            HeaderSource::Cel(_)
        ));
    }

    #[test]
    fn malformed_source_fails_deserialization() {
        assert!(
            serde_json::from_value::<HeaderSource>(json!({ "syntax": "cel", "source": "a +" }))
                .is_err()
        );
    }
}
