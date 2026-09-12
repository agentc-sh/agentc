// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use hcl::{
    Attribute, Block, Body, Expression, FuncCall, ObjectKey, Structure,
    expr::{
        BinaryOp, Conditional, ForExpr, Object, Operation, Traversal, TraversalOperator, UnaryOp,
    },
};

use crate::parser::{errors::ParserError, middleware::traits::FormatMiddleware};

trait RuntimeExpressionTransform {
    fn transform_before(expr: Expression) -> Result<Expression, ParserError> {
        Ok(expr)
    }

    fn transform_after(expr: Expression) -> Result<Expression, ParserError> {
        Ok(expr)
    }

    fn transform_expr(expr: Expression) -> Result<Expression, ParserError> {
        Self::transform_after(match Self::transform_before(expr)? {
            Expression::Array(values) => Expression::Array(
                values
                    .into_iter()
                    .map(Self::transform_expr)
                    .collect::<Result<_, _>>()?,
            ),
            Expression::Object(values) => Expression::Object(
                values
                    .into_iter()
                    .map(|(key, value)| {
                        Ok((
                            match key {
                                ObjectKey::Expression(expr) => {
                                    ObjectKey::Expression(Self::transform_expr(expr)?)
                                }
                                key => key,
                            },
                            Self::transform_expr(value)?,
                        ))
                    })
                    .collect::<Result<_, ParserError>>()?,
            ),
            Expression::Traversal(traversal) => Expression::Traversal(Box::new(Traversal {
                expr: Self::transform_expr(traversal.expr)?,
                operators: traversal
                    .operators
                    .into_iter()
                    .map(|operator| {
                        Ok(match operator {
                            TraversalOperator::Index(expr) => {
                                TraversalOperator::Index(Self::transform_expr(expr)?)
                            }
                            operator => operator,
                        })
                    })
                    .collect::<Result<_, ParserError>>()?,
            })),
            Expression::FuncCall(call) => Expression::FuncCall(Box::new(FuncCall {
                args: call
                    .args
                    .into_iter()
                    .map(Self::transform_expr)
                    .collect::<Result<_, _>>()?,
                ..*call
            })),
            Expression::Parenthesis(expr) => {
                Expression::Parenthesis(Box::new(Self::transform_expr(*expr)?))
            }
            Expression::Conditional(conditional) => {
                Expression::Conditional(Box::new(Conditional {
                    cond_expr: Self::transform_expr(conditional.cond_expr)?,
                    true_expr: Self::transform_expr(conditional.true_expr)?,
                    false_expr: Self::transform_expr(conditional.false_expr)?,
                }))
            }
            Expression::Operation(operation) => Expression::Operation(Box::new(match *operation {
                Operation::Unary(operation) => Operation::Unary(UnaryOp {
                    expr: Self::transform_expr(operation.expr)?,
                    ..operation
                }),
                Operation::Binary(operation) => Operation::Binary(BinaryOp {
                    lhs_expr: Self::transform_expr(operation.lhs_expr)?,
                    rhs_expr: Self::transform_expr(operation.rhs_expr)?,
                    ..operation
                }),
            })),
            Expression::ForExpr(for_expr) => Expression::ForExpr(Box::new(ForExpr {
                collection_expr: Self::transform_expr(for_expr.collection_expr)?,
                key_expr: for_expr
                    .key_expr
                    .map(Self::transform_expr)
                    .transpose()?,
                value_expr: Self::transform_expr(for_expr.value_expr)?,
                cond_expr: for_expr
                    .cond_expr
                    .map(Self::transform_expr)
                    .transpose()?,
                ..*for_expr
            })),
            expr => expr,
        })
    }

    fn transform_body(body: Body) -> Result<Body, ParserError> {
        body.into_iter()
            .map(|structure| match structure {
                Structure::Attribute(attr) => Ok(Structure::Attribute(Attribute::new(
                    attr.key,
                    Self::transform_expr(attr.expr)?,
                ))),
                Structure::Block(block) => Ok(Structure::Block(Block {
                    body: Self::transform_body(block.body)?,
                    ..block
                })),
            })
            .collect::<Result<_, ParserError>>()
    }
}

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

impl RuntimeExpressionTransform for RuntimeFunctionDeserialize {
    fn transform_before(expr: Expression) -> Result<Expression, ParserError> {
        match expr {
            Expression::FuncCall(call) if call.name.name.as_str() == "runtime" => {
                Self::transform_runtime(*call)
            }
            Expression::FuncCall(call) if call.name.name.as_str() == "secret" => {
                Self::transform_secret(*call)
            }
            expr => Ok(expr),
        }
    }
}

impl FormatMiddleware<Body> for RuntimeFunctionDeserialize {
    fn apply(&self, input: Body) -> Result<Body, ParserError> {
        Self::transform_body(input)
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

impl RuntimeExpressionTransform for RuntimeFunctionSerialize {
    fn transform_after(expr: Expression) -> Result<Expression, ParserError> {
        Ok(match expr {
            Expression::Object(object) => Self::transform_runtime_object(object),
            expr => expr,
        })
    }
}

impl FormatMiddleware<Body> for RuntimeFunctionSerialize {
    fn apply(&self, input: Body) -> Result<Body, ParserError> {
        Self::transform_body(input)
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
