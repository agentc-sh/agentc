// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    errors::DatabaseError,
    orm::{
        Condition, ConnectionTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder,
        QuerySelect, Select, Value as DatabaseValue,
    },
    query::{Alias, BinOper, Expr},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    pub fn to_order(&self) -> Order {
        match self {
            SortDirection::Asc => Order::Asc,
            SortDirection::Desc => Order::Desc,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CursorValue {
    Int(Option<i64>),
    Float(Option<f64>),
    Uint(Option<u64>),
    String(Option<String>),
    Bool(Option<bool>),
    DateTime(Option<DateTime<Utc>>),
    Uuid(Option<Uuid>),
}

impl CursorValue {
    pub fn to_database_value(&self) -> DatabaseValue {
        match self {
            CursorValue::Int(v) => (*v).into(),
            CursorValue::Float(v) => (*v).into(),
            CursorValue::Uint(v) => (*v).into(),
            CursorValue::String(v) => (*v).clone().into(),
            CursorValue::Bool(v) => (*v).into(),
            CursorValue::DateTime(v) => (*v).into(),
            CursorValue::Uuid(v) => (*v).into(),
        }
    }
}

pub trait ExtractCursorValue {
    fn extract_cursor_value(&self, field_name: &str) -> Result<CursorValue, DatabaseError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorField {
    pub name: String,
    pub direction: SortDirection,
    pub last_value: CursorValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    pub fields: Vec<CursorField>,
}

impl Cursor {
    pub fn encode(&self) -> String {
        bs58::encode(serde_json::to_vec(self).expect("Failed to serialize cursor")).into_string()
    }

    pub fn decode(c: &str) -> Option<Self> {
        serde_json::from_slice(&bs58::decode(c).into_vec().ok()?).ok()
    }
}

#[derive(Debug, Clone)]
pub struct CursorPage<T> {
    pub items: Vec<T>,
    pub count: u64,
    pub next_page: Option<String>,
}

impl<T> Default for CursorPage<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            count: 0,
            next_page: None,
        }
    }
}

pub trait CursorPaginatorExt<E>
where
    E: EntityTrait,
{
    fn cursor_paginate(self) -> CursorPaginationBuilder<E>;
}

impl<E> CursorPaginatorExt<E> for Select<E>
where
    E: EntityTrait,
{
    fn cursor_paginate(self) -> CursorPaginationBuilder<E> {
        CursorPaginationBuilder::new(self)
    }
}

pub struct CursorPaginationBuilder<E>
where
    E: EntityTrait,
{
    query: Select<E>,
    per_page: Option<u64>,
    cursor: Option<Cursor>,
    sorts: Vec<(String, SortDirection)>,
}

impl<E> CursorPaginationBuilder<E>
where
    E: EntityTrait,
{
    pub fn new(query: Select<E>) -> Self {
        Self {
            query,
            per_page: Some(10),
            cursor: None,
            sorts: Vec::new(),
        }
    }

    fn apply_cursor_filters(mut self) -> Self {
        let cursor = match &self.cursor {
            Some(c) => c,
            None => return self,
        };

        let mut condition = Condition::any();

        for i in 0..cursor.fields.len() {
            let mut field_condition = Condition::all();

            for j in 0..i {
                let field = &cursor.fields[j];

                field_condition = field_condition.add(
                    Expr::col((E::default(), Alias::new(&field.name)))
                        .eq(field.last_value.to_database_value()),
                );
            }

            let current_field = &cursor.fields[i];
            let comparison = match current_field.direction {
                SortDirection::Asc => BinOper::GreaterThan,
                SortDirection::Desc => BinOper::SmallerThan,
            };

            field_condition = field_condition.add(
                Expr::col((E::default(), Alias::new(&current_field.name))).binary(
                    comparison,
                    current_field
                        .last_value
                        .to_database_value(),
                ),
            );

            condition = condition.add(field_condition);
        }

        self.query = self.query.filter(condition);
        self
    }

    pub fn per_page(mut self, per_page: Option<u64>) -> Self {
        self.per_page = per_page;
        self
    }

    pub fn cursor(mut self, cursor: Option<String>) -> Self {
        if let Some(cursor_str) = cursor {
            self.cursor = Cursor::decode(&cursor_str);
        }
        self
    }

    pub fn decoded_cursor(mut self, cursor: Option<Cursor>) -> Self {
        self.cursor = cursor;
        self
    }

    pub fn sort_by(mut self, field: impl Into<String>, direction: SortDirection) -> Self {
        self.sorts
            .push((field.into(), direction));
        self
    }

    pub fn sort_by_columns(
        mut self,
        sorts: impl IntoIterator<Item = (impl Into<String>, SortDirection)>,
    ) -> Self {
        self.sorts.extend(
            sorts
                .into_iter()
                .map(|(field, direction)| (field.into(), direction)),
        );
        self
    }

    pub fn sort_asc(self, field: impl Into<String>) -> Self {
        self.sort_by(field, SortDirection::Asc)
    }

    pub fn sort_desc(self, field: impl Into<String>) -> Self {
        self.sort_by(field, SortDirection::Desc)
    }

    pub async fn execute<C>(mut self, database: &C) -> Result<CursorPage<E::Model>, DatabaseError>
    where
        C: ConnectionTrait,
        E::Model: ExtractCursorValue + Send + Sync + 'static,
    {
        for (field, direction) in &self.sorts {
            self.query = self
                .query
                .order_by(Expr::col((E::default(), Alias::new(field))), direction.to_order());
        }

        let count = self
            .query
            .clone()
            .count(database)
            .await
            .map_err(DatabaseError::from)?;

        self = self.apply_cursor_filters();

        // let items = self
        //     .query
        //     .limit(self.per_page + 1)
        //     .all(database)
        //     .await
        //     .map_err(DatabaseError::from)?;

        let query = if let Some(per_page) = self.per_page {
            self.query.limit(per_page + 1)
        } else {
            self.query
        };

        let items = query
            .all(database)
            .await
            .map_err(DatabaseError::from)?;

        let has_next_page = items.len() as u64 > self.per_page.unwrap_or(u64::MAX);
        let mut page_items = items;

        if has_next_page {
            page_items.pop();
        }

        let next_page = if has_next_page {
            page_items
                .last()
                .map(|last_item| {
                    Ok::<Cursor, DatabaseError>(Cursor {
                        fields: self
                            .sorts
                            .iter()
                            .map(|(field, direction)| {
                                Ok(CursorField {
                                    name: field.clone(),
                                    direction: direction.clone(),
                                    last_value: last_item.extract_cursor_value(field)?,
                                })
                            })
                            .collect::<Result<_, DatabaseError>>()?,
                    })
                })
                .transpose()?
                .map(|cursor| cursor.encode())
        } else {
            None
        };

        Ok(CursorPage { items: page_items, count, next_page })
    }
}
