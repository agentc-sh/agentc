use hcl::{
    Attribute, Block, Body, Expression, FuncCall, ObjectKey, Structure,
    expr::{BinaryOp, Conditional, ForExpr, Operation, Traversal, TraversalOperator, UnaryOp},
};

use crate::parser::errors::ParserError;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpressionPosition {
    Value,
    ObjectKey,
}

#[derive(Clone, Copy)]
pub(crate) enum VisitOrder {
    Before,
    After,
}

pub(crate) struct ExpressionVisitor<F> {
    order: VisitOrder,
    transform: F,
}

impl<F> ExpressionVisitor<F>
where
    F: Fn(Expression, ExpressionPosition) -> Result<Expression, ParserError>,
{
    pub(crate) fn new(order: VisitOrder, transform: F) -> Self {
        Self { order, transform }
    }

    fn visit_expr(
        &self,
        expr: Expression,
        position: ExpressionPosition,
    ) -> Result<Expression, ParserError> {
        let expr = match self.order {
            VisitOrder::Before => (self.transform)(expr, position)?,
            VisitOrder::After => expr,
        };

        let expr = match expr {
            Expression::Array(values) => Expression::Array(
                values
                    .into_iter()
                    .map(|value| self.visit_expr(value, position))
                    .collect::<Result<_, _>>()?,
            ),
            Expression::Object(values) => Expression::Object(
                values
                    .into_iter()
                    .map(|(key, value)| {
                        Ok((
                            match key {
                                ObjectKey::Expression(expr) => ObjectKey::Expression(
                                    self.visit_expr(expr, ExpressionPosition::ObjectKey)?,
                                ),
                                key => key,
                            },
                            self.visit_expr(value, position)?,
                        ))
                    })
                    .collect::<Result<_, ParserError>>()?,
            ),
            Expression::Traversal(traversal) => Expression::Traversal(Box::new(Traversal {
                expr: self.visit_expr(traversal.expr, position)?,
                operators: traversal
                    .operators
                    .into_iter()
                    .map(|operator| {
                        Ok(match operator {
                            TraversalOperator::Index(expr) => {
                                TraversalOperator::Index(self.visit_expr(expr, position)?)
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
                    .map(|arg| self.visit_expr(arg, position))
                    .collect::<Result<_, _>>()?,
                ..*call
            })),
            Expression::Parenthesis(expr) => {
                Expression::Parenthesis(Box::new(self.visit_expr(*expr, position)?))
            }
            Expression::Conditional(conditional) => {
                Expression::Conditional(Box::new(Conditional {
                    cond_expr: self.visit_expr(conditional.cond_expr, position)?,
                    true_expr: self.visit_expr(conditional.true_expr, position)?,
                    false_expr: self.visit_expr(conditional.false_expr, position)?,
                }))
            }
            Expression::Operation(operation) => Expression::Operation(Box::new(match *operation {
                Operation::Unary(operation) => Operation::Unary(UnaryOp {
                    expr: self.visit_expr(operation.expr, position)?,
                    ..operation
                }),
                Operation::Binary(operation) => Operation::Binary(BinaryOp {
                    lhs_expr: self.visit_expr(operation.lhs_expr, position)?,
                    rhs_expr: self.visit_expr(operation.rhs_expr, position)?,
                    ..operation
                }),
            })),
            Expression::ForExpr(for_expr) => Expression::ForExpr(Box::new(ForExpr {
                collection_expr: self.visit_expr(for_expr.collection_expr, position)?,
                key_expr: for_expr
                    .key_expr
                    .map(|expr| self.visit_expr(expr, ExpressionPosition::ObjectKey))
                    .transpose()?,
                value_expr: self.visit_expr(for_expr.value_expr, position)?,
                cond_expr: for_expr
                    .cond_expr
                    .map(|expr| self.visit_expr(expr, position))
                    .transpose()?,
                ..*for_expr
            })),
            expr => expr,
        };

        match self.order {
            VisitOrder::Before => Ok(expr),
            VisitOrder::After => (self.transform)(expr, position),
        }
    }

    pub(crate) fn visit_body(&self, body: Body) -> Result<Body, ParserError> {
        body.into_iter()
            .map(|structure| match structure {
                Structure::Attribute(attr) => Ok(Structure::Attribute(Attribute::new(
                    attr.key,
                    self.visit_expr(attr.expr, ExpressionPosition::Value)?,
                ))),
                Structure::Block(block) => Ok(Structure::Block(Block {
                    body: self.visit_body(block.body)?,
                    ..block
                })),
            })
            .collect::<Result<_, ParserError>>()
    }
}
