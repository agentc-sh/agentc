// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::types::RunStatus;

#[derive(Debug, Clone, Default)]
pub struct FindRunParams {
    pub per_page: Option<u64>,
    pub page: Option<String>,
    pub tenant_ids: Option<Vec<String>>,
    pub ids: Option<Vec<Uuid>>,
    pub session_ids: Option<Vec<Uuid>>,
    pub statuses: Option<Vec<RunStatus>>,
    pub created_before: Option<DateTime<Utc>>,
    pub created_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
}

impl FindRunParams {
    pub fn new() -> Self {
        Self { per_page: Some(10), ..Default::default() }
    }

    pub fn per_page(mut self, per_page: impl Into<u64>) -> Self {
        self.per_page = Some(per_page.into());
        self
    }

    pub fn maybe_per_page(mut self, per_page: Option<impl Into<u64>>) -> Self {
        self.per_page = per_page.map(Into::into);
        self
    }

    pub fn no_limit(mut self) -> Self {
        self.per_page = None;
        self
    }

    pub fn page(mut self, page: impl Into<String>) -> Self {
        self.page = Some(page.into());
        self
    }

    pub fn tenant_ids(mut self, tenant_ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tenant_ids = Some(
            tenant_ids
                .into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    pub fn ids(mut self, ids: impl IntoIterator<Item = impl Into<Uuid>>) -> Self {
        self.ids = Some(
            ids.into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    pub fn session_ids(mut self, session_ids: impl IntoIterator<Item = impl Into<Uuid>>) -> Self {
        self.session_ids = Some(
            session_ids
                .into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    pub fn statuses(mut self, statuses: impl IntoIterator<Item = impl Into<RunStatus>>) -> Self {
        self.statuses = Some(
            statuses
                .into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    pub fn created_before(mut self, created_before: impl Into<DateTime<Utc>>) -> Self {
        self.created_before = Some(created_before.into());
        self
    }

    pub fn created_after(mut self, created_after: impl Into<DateTime<Utc>>) -> Self {
        self.created_after = Some(created_after.into());
        self
    }

    pub fn updated_before(mut self, updated_before: impl Into<DateTime<Utc>>) -> Self {
        self.updated_before = Some(updated_before.into());
        self
    }

    pub fn updated_after(mut self, updated_after: impl Into<DateTime<Utc>>) -> Self {
        self.updated_after = Some(updated_after.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct DeleteRunParams {
    pub tenant_id: String,
    pub ids: Vec<Uuid>,
}

impl DeleteRunParams {
    pub fn new(
        tenant_id: impl Into<String>,
        ids: impl IntoIterator<Item = impl Into<Uuid>>,
    ) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            ids: ids
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}
