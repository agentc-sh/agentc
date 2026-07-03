// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use cel::{Context as CelContext, Program, Value as CelValue};
use serde::Serialize;

use crate::generator::errors::GeneratorError;

/// Evaluates CEL boolean expressions against a serialized context value.
pub(crate) struct ConditionEvaluator {
    cel: CelContext<'static>,
}

impl ConditionEvaluator {
    /// Build an evaluator from any serializable value.
    pub(crate) fn new<T: Serialize>(data: &T) -> Result<Self, GeneratorError> {
        let mut cel = CelContext::default();
        cel.add_variable("ctx", data)
            .map_err(|e| GeneratorError::ContextSerialization(e.to_string()))?;

        Ok(Self { cel })
    }

    /// Evaluate a CEL expression, returning its boolean result.
    pub(crate) fn evaluate(
        &self,
        block_id: &str,
        expression: &str,
    ) -> Result<bool, GeneratorError> {
        match Program::compile(expression)
            .map_err(|e| GeneratorError::ConditionParseFailed {
                block_id: block_id.to_string(),
                message: e.to_string(),
            })?
            .execute(&self.cel)
            .map_err(|e| GeneratorError::ConditionEvalFailed {
                block_id: block_id.to_string(),
                source: e,
            })? {
            CelValue::Bool(b) => Ok(b),
            _ => Err(GeneratorError::ConditionNotBoolean { block_id: block_id.to_string() }),
        }
    }
}
