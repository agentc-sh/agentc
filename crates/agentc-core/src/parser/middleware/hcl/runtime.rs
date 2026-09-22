// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use hcl::{Body, Expression, FuncCall, ObjectKey, expr::Object};

use crate::parser::{
    errors::ParserError,
    middleware::{
        hcl::expression::{ExpressionVisitor, VisitOrder},
        traits::FormatMiddleware,
    },
};

pub struct RuntimeFunctionDeserialize;

impl RuntimeFunctionDeserialize {
    fn transform_runtime(call: FuncCall) -> Result<Expression, ParserError> {
        Ok(match call.args.as_slice() {
            [env_param] => hcl::expression!({ "env" = (env_param.clone()) }),
            [env_param, default_param] => hcl::expression!({
                "env" = (env_param.clone()),
                "default" = (default_param.clone())
            }),
            _ => {
                return Err(ParserError::InvalidExpression(
                    "runtime() takes 1 or 2 arguments".to_string(),
                ));
            }
        })
    }

    fn transform_secret(call: FuncCall) -> Result<Expression, ParserError> {
        match call.args.as_slice() {
            [Expression::FuncCall(inner)] if inner.name.name.as_str() == "runtime" => {
                match Self::transform_runtime(*inner.clone())? {
                    Expression::Object(mut object) => {
                        object.insert(ObjectKey::from("secret"), Expression::Bool(true));

                        Ok(Expression::Object(object))
                    }
                    _ => unreachable!(),
                }
            }
            [_] => Err(ParserError::InvalidExpression(
                "secret() must wrap a runtime() call".to_string(),
            )),
            _ => {
                Err(ParserError::InvalidExpression("secret() takes exactly 1 argument".to_string()))
            }
        }
    }
}

impl FormatMiddleware<Body> for RuntimeFunctionDeserialize {
    fn apply(&self, input: Body) -> Result<Body, ParserError> {
        ExpressionVisitor::new(VisitOrder::Before, |expr, _| match expr {
            Expression::FuncCall(call) if call.name.name.as_str() == "runtime" => {
                Self::transform_runtime(*call)
            }
            Expression::FuncCall(call) if call.name.name.as_str() == "secret" => {
                Self::transform_secret(*call)
            }
            expr => Ok(expr),
        })
        .visit_body(input)
    }
}

pub struct RuntimeFunctionSerialize;

impl RuntimeFunctionSerialize {
    fn transform_runtime_object(object: Object<ObjectKey, Expression>) -> Expression {
        let env_key = ObjectKey::from("env");
        let default_key = ObjectKey::from("default");
        let secret_key = ObjectKey::from("secret");

        if object
            .keys()
            .any(|key| key != &env_key && key != &default_key && key != &secret_key)
        {
            return Expression::Object(object);
        }

        let Some(Expression::String(env)) = object.get(&env_key) else {
            return Expression::Object(object);
        };

        let secret = match object.get(&secret_key) {
            Some(Expression::Bool(secret)) => *secret,
            Some(_) => return Expression::Object(object),
            None => false,
        };

        let mut builder = FuncCall::builder("runtime").arg(Expression::String(env.clone()));

        if let Some(default) = object.get(&default_key) {
            builder = builder.arg(default.clone());
        }

        let runtime = Expression::FuncCall(Box::new(builder.build()));

        if secret {
            Expression::FuncCall(Box::new(
                FuncCall::builder("secret")
                    .arg(runtime)
                    .build(),
            ))
        } else {
            runtime
        }
    }
}

impl FormatMiddleware<Body> for RuntimeFunctionSerialize {
    fn apply(&self, input: Body) -> Result<Body, ParserError> {
        ExpressionVisitor::new(VisitOrder::After, |expr, _| {
            Ok(match expr {
                Expression::Object(object) => Self::transform_runtime_object(object),
                expr => expr,
            })
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
}
