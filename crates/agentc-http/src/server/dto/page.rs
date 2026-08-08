// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use axum::{http::StatusCode, response::IntoResponse, response::Json, response::Response};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use agentc_domain::types::Page;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponseDTO<T> {
    pub items: Vec<T>,
    pub count: u64,
    pub next_page: Option<String>,
}

impl<T> PaginatedResponseDTO<T> {
    pub fn builder(items: Vec<T>) -> PaginatedResponseBuilder<T> {
        PaginatedResponseBuilder::new(items)
    }

    pub fn from_result<U, F>(page: Page<T>, f: F) -> PaginatedResponseDTO<U>
    where
        F: FnMut(T) -> U,
    {
        PaginatedResponseDTO {
            items: page.items.into_iter().map(f).collect(),
            count: page.count,
            next_page: page.next_page,
        }
    }
}

pub struct PaginatedResponseBuilder<T> {
    items: Vec<T>,
    count: u64,
    next_page: Option<String>,
}

impl<T> PaginatedResponseBuilder<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self { items, count: 0, next_page: None }
    }

    pub fn with_count(mut self, count: u64) -> Self {
        self.count = count;
        self
    }

    pub fn with_next_page(mut self, next_page: Option<String>) -> Self {
        self.next_page = next_page;
        self
    }

    pub fn build(self) -> PaginatedResponseDTO<T> {
        PaginatedResponseDTO {
            items: self.items,
            count: self.count,
            next_page: self.next_page,
        }
    }
}

impl<T, D> From<Page<T>> for PaginatedResponseDTO<D>
where
    D: From<T>,
{
    fn from(page: Page<T>) -> PaginatedResponseDTO<D> {
        PaginatedResponseDTO::builder(
            page.items
                .into_iter()
                .map(D::from)
                .collect::<Vec<D>>(),
        )
        .with_count(page.count)
        .with_next_page(page.next_page)
        .build()
    }
}

impl<T> IntoResponse for PaginatedResponseDTO<T>
where
    T: Serialize + ToSchema,
{
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
