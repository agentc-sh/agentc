// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use hcl::{Body, Expression, FuncCall};
use serde::Deserialize;

use agentc_blocks::types::RuntimeValueWire;

use crate::parser::{
    errors::ParserError,
    middleware::{
        hcl::expression::{ExpressionVisitor, VisitOrder},
        traits::FormatMiddleware,
    },
};

pub struct RuntimeFunctionDeserialize;

impl RuntimeFunctionDeserialize {
    fn transform_runtime(call: FuncCall, secret: bool) -> Result<Expression, ParserError> {
        let mut args = call.args.into_iter();

        let Some(env) = args.next() else {
            return Err(ParserError::InvalidExpression(
                "runtime() takes 1 or 2 arguments".to_string(),
            ));
        };

        let default = args.next();

        if args.next().is_some() {
            return Err(ParserError::InvalidExpression(
                "runtime() takes 1 or 2 arguments".to_string(),
            ));
        }

        Ok(hcl::to_expression(RuntimeValueWire::new(
            env, default, secret,
        ))?)
    }
}

impl FormatMiddleware<Body> for RuntimeFunctionDeserialize {
    fn apply(&self, input: Body) -> Result<Body, ParserError> {
        ExpressionVisitor::new(VisitOrder::Before, |expr, _| match expr {
            Expression::FuncCall(call) if call.name.name.as_str() == "runtime" => {
                Self::transform_runtime(*call, false)
            }
            Expression::FuncCall(call) if call.name.name.as_str() == "secret" => {
                let mut args = call.args.into_iter();

                match (args.next(), args.next()) {
                    (Some(Expression::FuncCall(inner)), None)
                        if inner.name.name.as_str() == "runtime" =>
                    {
                        Self::transform_runtime(*inner, true)
                    }
                    (Some(_), None) => Err(ParserError::InvalidExpression(
                        "secret() must wrap a runtime() call".to_string(),
                    )),
                    _ => Err(ParserError::InvalidExpression(
                        "secret() takes exactly 1 argument".to_string(),
                    )),
                }
            }
            expr => Ok(expr),
        })
        .visit_body(input)
    }
}

pub struct RuntimeFunctionSerialize;

impl FormatMiddleware<Body> for RuntimeFunctionSerialize {
    fn apply(&self, input: Body) -> Result<Body, ParserError> {
        ExpressionVisitor::new(VisitOrder::After, |expr, _| {
            let Expression::Object(object) = expr else {
                return Ok(expr);
            };

            if object.len() != 1 {
                return Ok(Expression::Object(object));
            }

            let Ok(wire) = RuntimeValueWire::<String, Expression>::deserialize(
                Expression::Object(object.clone()),
            ) else {
                return Ok(Expression::Object(object));
            };

            let (env, default, secret) = wire.into_parts();
            let mut builder = FuncCall::builder("runtime").arg(Expression::String(env));

            if let Some(default) = default {
                builder = builder.arg(default);
            }

            let runtime = Expression::FuncCall(Box::new(builder.build()));

            if secret {
                Ok(Expression::FuncCall(Box::new(
                    FuncCall::builder("secret")
                        .arg(runtime)
                        .build(),
                )))
            } else {
                Ok(runtime)
            }
        })
        .visit_body(input)
    }
}

#[cfg(test)]
mod tests {
    use agentc_blocks::types::RuntimeValue;
    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::parser::SpecFormat;

    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    struct RuntimeFixture {
        direct: RuntimeValue<String>,
        nested: RuntimeNestedFixture,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    struct RuntimeNestedFixture {
        values: Vec<RuntimeValue<String>>,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    struct OrdinaryFixture {
        nested: OrdinaryNestedFixture,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    struct OrdinaryNestedFixture {
        env: String,
        description: String,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    struct OrdinaryRuntimeFixture {
        value: RuntimeValue<OrdinaryNestedFixture>,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    struct MarkerFixture {
        value: RuntimeValue<String>,
    }

    fn format() -> SpecFormat {
        SpecFormat::hcl()
            .with_hcl_deserialize_middleware(RuntimeFunctionDeserialize)
            .with_hcl_serialize_middleware(RuntimeFunctionSerialize)
    }

    #[test]
    fn nested_runtime_values_round_trip() {
        let fixture = format()
            .deserialize_string::<RuntimeFixture>(
                r#"
direct = runtime("DIRECT")
nested = {
  values = [
    runtime("FIRST", "first"),
    secret(runtime("SECOND", "second"))
  ]
}
"#,
            )
            .unwrap();

        assert_eq!(
            fixture,
            RuntimeFixture {
                direct: RuntimeValue::required_runtime("DIRECT"),
                nested: RuntimeNestedFixture {
                    values: vec![
                        RuntimeValue::default_runtime("FIRST", "first".to_string(),),
                        RuntimeValue::secret_default_runtime("SECOND", "second".to_string(),),
                    ],
                },
            }
        );

        assert_eq!(
            format()
                .deserialize_string::<RuntimeFixture>(
                    &format()
                        .serialize_string(&fixture)
                        .unwrap()
                )
                .unwrap(),
            fixture
        );
    }

    #[test]
    fn nested_runtime_rejects_invalid_argument_count() {
        let error = format()
            .deserialize_string::<serde_json::Value>(
                r#"
nested = {
  value = runtime()
}
"#,
            )
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("runtime() takes 1 or 2 arguments")
        );
    }

    #[test]
    fn nested_secret_rejects_non_runtime_argument() {
        let error = format()
            .deserialize_string::<serde_json::Value>(
                r#"
nested = {
  value = secret("VALUE")
}
"#,
            )
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("secret() must wrap a runtime() call")
        );
    }

    #[test]
    fn ordinary_nested_env_object_round_trips() {
        let fixture = OrdinaryFixture {
            nested: OrdinaryNestedFixture {
                env: "production".to_string(),
                description: "deployment environment".to_string(),
            },
        };

        assert_eq!(
            format()
                .deserialize_string::<OrdinaryFixture>(
                    &format()
                        .serialize_string(&fixture)
                        .unwrap()
                )
                .unwrap(),
            fixture
        );
    }

    #[test]
    fn ordinary_env_map_remains_a_constant() {
        let fixture = format()
            .deserialize_string::<OrdinaryRuntimeFixture>(
                r#"
value = {
  env = "production"
  description = "deployment environment"
}
"#,
            )
            .unwrap();

        assert_eq!(
            fixture,
            OrdinaryRuntimeFixture {
                value: RuntimeValue::Constant(OrdinaryNestedFixture {
                    env: "production".to_string(),
                    description: "deployment environment".to_string(),
                }),
            }
        );

        let serialized = format().serialize_string(&fixture).unwrap();

        assert!(!serialized.contains("runtime("));
        assert_eq!(
            format()
                .deserialize_string::<OrdinaryRuntimeFixture>(&serialized)
                .unwrap(),
            fixture
        );
    }

    #[test]
    fn explicit_marker_with_identifier_fields_round_trips() {
        let fixture = format()
            .deserialize_string::<MarkerFixture>(
                r#"
value = {
  "$runtime" = {
    env = "PORT"
    default = "8080"
    secret = true
  }
}
"#,
            )
            .unwrap();

        assert_eq!(
            fixture,
            MarkerFixture {
                value: RuntimeValue::secret_default_runtime("PORT", "8080".to_string()),
            }
        );

        let serialized = format().serialize_string(&fixture).unwrap();

        assert!(serialized.contains("secret(runtime("));
        assert_eq!(
            format()
                .deserialize_string::<MarkerFixture>(&serialized)
                .unwrap(),
            fixture
        );
    }

    #[test]
    fn marker_with_sibling_is_rejected_in_both_orders() {
        for source in [
            r#"
value = {
  "$runtime" = { env = "PORT" }
  other = "value"
}
"#,
            r#"
value = {
  other = "value"
  "$runtime" = { env = "PORT" }
}
"#,
        ] {
            assert!(format().deserialize_string::<MarkerFixture>(source).is_err());
        }
    }
}
