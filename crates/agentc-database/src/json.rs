// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::ops::Deref;

use crate::{
    orm::{ColumnTrait, DatabaseBackend, TryGetableFromJson, Value},
    query::{ArrayType, ColumnType, Expr, Nullable, SimpleExpr, ValueType},
};

/// Wrapper type for JSON-serialized data in the database
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Json<T>(pub T);

impl<T> Json<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn as_inner(&self) -> &T {
        &self.0
    }

    pub fn as_inner_mut(&mut self) -> &mut T {
        &mut self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<T> for Json<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> TryGetableFromJson for Json<T> where T: for<'de> Deserialize<'de> {}

impl<T> From<Json<T>> for Value
where
    T: for<'de> Serialize,
{
    fn from(source: Json<T>) -> Self {
        Value::Json(Some(Box::new(
            serde_json::to_value(&source.0).expect("Failed to serialize Json value"),
        )))
    }
}

impl<T> ValueType for Json<T>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    fn try_from(v: Value) -> Result<Self, sea_orm::sea_query::ValueTypeErr> {
        match v {
            Value::Json(Some(json)) => Ok(Json(
                serde_json::from_value(*json).map_err(|_| sea_orm::sea_query::ValueTypeErr)?,
            )),
            _ => Err(sea_orm::sea_query::ValueTypeErr),
        }
    }

    fn type_name() -> String {
        stringify!(Json<T>).to_owned()
    }

    fn array_type() -> ArrayType {
        ArrayType::Json
    }

    fn column_type() -> ColumnType {
        ColumnType::Json
    }
}

impl<T> Nullable for Json<T>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    fn null() -> Value {
        Value::Json(None)
    }
}

/// Extension trait for JSON column operations
pub trait JsonColumnExt {
    fn json_key_eq<K, V>(&self, backend: &DatabaseBackend, key: K, value: V) -> SimpleExpr
    where
        K: Into<String>,
        V: Into<Value>;
}

impl<C: ColumnTrait> JsonColumnExt for C {
    fn json_key_eq<K, V>(&self, backend: &DatabaseBackend, key: K, value: V) -> SimpleExpr
    where
        K: Into<String>,
        V: Into<Value>,
    {
        match backend {
            DatabaseBackend::Sqlite => Expr::cust_with_values(
                format!("json_extract({}, ?)", self.to_string()),
                vec![format!("$.{}", key.into())],
            )
            .eq(value),
            DatabaseBackend::Postgres => {
                Expr::cust_with_values(format!("{} ->> $1", self.to_string()), vec![key.into()])
                    .eq(value)
            }
            _ => panic!("Unsupported backend for JSON column extension"),
        }
    }
}
