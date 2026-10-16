// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use hcl::{Body, Expression};

use agentc_blocks::types::ExpressionValueWire;

use crate::parser::{
    errors::ParserError,
    middleware::{
        hcl::visitor::{ExpressionPosition, ExpressionVisitor, VisitOrder},
        traits::FormatMiddleware,
    },
};

pub struct ExpressionFunctionDeserialize;

impl FormatMiddleware<Body> for ExpressionFunctionDeserialize {
    fn apply(&self, input: Body) -> Result<Body, ParserError> {
        ExpressionVisitor::new(VisitOrder::Before, |expr, position| match expr {
            Expression::FuncCall(call)
                if call.name.name.as_str() == "expression"
                    && position == ExpressionPosition::Value =>
            {
                match call.args.as_slice() {
                    [Expression::String(source)] => {
                        Ok(hcl::to_expression(ExpressionValueWire::new(source.clone()))?)
                    }
                    _ => Err(ParserError::InvalidExpression(
                        "expression() requires one string".to_string(),
                    )),
                }
            }
            expr => Ok(expr),
        })
        .visit_body(input)
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::parser::SpecFormat;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Fixture {
        value: serde_json::Value,
    }

    #[test]
    fn rewrites_into_a_marker() {
        assert_eq!(
            SpecFormat::hcl()
                .with_hcl_deserialize_middleware(ExpressionFunctionDeserialize)
                .deserialize_string::<Fixture>(r#"value = expression("tenant_id")"#)
                .unwrap(),
            Fixture {
                value: serde_json::json!({ "$expression": "tenant_id" }),
            }
        );
    }

    #[test]
    fn rejects_non_string_argument() {
        assert!(
            SpecFormat::hcl()
                .with_hcl_deserialize_middleware(ExpressionFunctionDeserialize)
                .deserialize_string::<Fixture>(r#"value = expression(1)"#)
                .is_err()
        );
    }
}
