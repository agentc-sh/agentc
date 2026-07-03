// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use hcl::{Attribute, Block, Body, Expression, FuncCall, ObjectKey, Structure};

use crate::parser::{errors::ParserError, middleware::traits::FormatMiddleware};

pub struct RuntimeFunctionDeserialize;

impl RuntimeFunctionDeserialize {
    fn transform_expr(expr: Expression) -> Result<Expression, ParserError> {
        match expr {
            Expression::FuncCall(call) if call.name.name.as_str() == "runtime" => {
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
            Expression::FuncCall(call) if call.name.name.as_str() == "secret" => {
                match call.args.as_slice() {
                    [inner] => match Self::transform_expr(inner.clone())? {
                        Expression::Object(mut obj) => {
                            obj.insert(ObjectKey::from("secret"), Expression::Bool(true));
                            Ok(Expression::Object(obj))
                        }
                        _ => Err(ParserError::InvalidExpression(
                            "secret() must wrap a runtime() call".to_string(),
                        )),
                    },
                    _ => Err(ParserError::InvalidExpression(
                        "secret() takes exactly 1 argument".to_string(),
                    )),
                }
            }
            other => Ok(other),
        }
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

impl FormatMiddleware<Body> for RuntimeFunctionDeserialize {
    fn apply(&self, input: Body) -> Result<Body, ParserError> {
        Self::transform_body(input)
    }
}

pub struct RuntimeFunctionSerialize;

impl RuntimeFunctionSerialize {
    fn transform_expr(expr: Expression) -> Result<Expression, ParserError> {
        match expr {
            Expression::Object(obj) => {
                match (
                    obj.get(&ObjectKey::from("env"))
                        .cloned(),
                    obj.get(&ObjectKey::from("default"))
                        .cloned(),
                    obj.get(&ObjectKey::from("secret"))
                        .cloned(),
                ) {
                    (Some(env_val), default_val, secret_val) => {
                        let mut builder = FuncCall::builder("runtime").arg(env_val);

                        if let Some(d) = default_val {
                            builder = builder.arg(d);
                        }

                        let runtime_expr = Expression::FuncCall(Box::new(builder.build()));

                        Ok(match secret_val {
                            Some(Expression::Bool(true)) => Expression::FuncCall(Box::new(
                                FuncCall::builder("secret")
                                    .arg(runtime_expr)
                                    .build(),
                            )),
                            _ => runtime_expr,
                        })
                    }
                    _ => Ok(Expression::Object(obj)),
                }
            }
            other => Ok(other),
        }
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

impl FormatMiddleware<Body> for RuntimeFunctionSerialize {
    fn apply(&self, input: Body) -> Result<Body, ParserError> {
        Self::transform_body(input)
    }
}
