// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use uuid::Uuid;

use agentc_domain::{
    repository::session::params::FindSessionParams as RepoFindSessionParams, types::Session,
};

#[derive(Debug, Clone)]
pub struct CreateSessionParams {
    pub tenant_id: String,
    pub id: Uuid,
}

impl CreateSessionParams {
    pub fn new(tenant_id: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            id: Uuid::new_v4(),
        }
    }

    pub fn with_id(mut self, id: impl Into<Uuid>) -> Self {
        self.id = id.into();
        self
    }

    pub fn to_entity(&self) -> Session {
        Session {
            id: self.id,
            tenant_id: self.tenant_id.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Default for CreateSessionParams {
    fn default() -> Self {
        Self {
            tenant_id: String::new(),
            id: Uuid::new_v4(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionResponse {
    pub id: Uuid,
    pub tenant_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SessionResponse {
    pub fn from_entity(session: &Session) -> Self {
        Self {
            id: session.id,
            tenant_id: session.tenant_id.clone(),
            created_at: session.created_at,
            updated_at: session.updated_at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FindSessionParams {
    pub per_page: Option<u64>,
    pub page: Option<String>,
    pub tenant_ids: Option<Vec<String>>,
    pub ids: Option<Vec<Uuid>>,
    pub created_before: Option<DateTime<Utc>>,
    pub created_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
}

impl FindSessionParams {
    pub fn new() -> Self {
        Self {
            per_page: Some(10),
            page: None,
            tenant_ids: None,
            ids: None,
            created_before: None,
            created_after: None,
            updated_before: None,
            updated_after: None,
        }
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

impl Default for FindSessionParams {
    fn default() -> Self {
        Self::new()
    }
}

impl From<FindSessionParams> for RepoFindSessionParams {
    fn from(params: FindSessionParams) -> Self {
        Self {
            per_page: params.per_page,
            page: params.page,
            tenant_ids: params.tenant_ids,
            ids: params.ids,
            created_before: params.created_before,
            created_after: params.created_after,
            updated_before: params.updated_before,
            updated_after: params.updated_after,
        }
    }
}
