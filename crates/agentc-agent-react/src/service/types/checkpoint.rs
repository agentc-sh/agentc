// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use agentc_domain::{
    repository::checkpoint_record::params::FindCheckpointRecordParams as RepoFindCheckpointRecordParams,
    types::{CheckpointReason, CheckpointRecord, RunStatus},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointResponse {
    pub id: Uuid,
    pub session_id: Uuid,
    pub run_id: Uuid,
    pub node: String,
    pub status: RunStatus,
    pub reason: CheckpointReason,
    pub parent_checkpoint_id: Option<Uuid>,
    pub metadata: Option<Value>,
    pub created_at: DateTime<Utc>,
}

impl CheckpointResponse {
    pub fn from_entity(entity: &CheckpointRecord) -> Self {
        Self {
            id: entity.id,
            session_id: entity.session_id,
            run_id: entity.run_id,
            node: entity.node.clone(),
            status: entity.status,
            reason: entity.reason,
            parent_checkpoint_id: entity.parent_checkpoint_id,
            metadata: entity.metadata.clone(),
            created_at: entity.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindCheckpointParams {
    pub per_page: Option<u64>,
    pub page: Option<String>,
    pub tenant_ids: Option<Vec<String>>,
    pub ids: Option<Vec<Uuid>>,
    pub session_ids: Option<Vec<Uuid>>,
    pub run_ids: Option<Vec<Uuid>>,
    pub reasons: Option<Vec<CheckpointReason>>,
    pub statuses: Option<Vec<RunStatus>>,
    pub created_before: Option<DateTime<Utc>>,
    pub created_after: Option<DateTime<Utc>>,
}

impl FindCheckpointParams {
    pub fn new() -> Self {
        Self {
            per_page: Some(10),
            page: None,
            tenant_ids: None,
            ids: None,
            session_ids: None,
            run_ids: None,
            reasons: None,
            statuses: None,
            created_before: None,
            created_after: None,
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

    pub fn tenant_ids(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tenant_ids = Some(
            ids.into_iter()
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

    pub fn session_ids(mut self, ids: impl IntoIterator<Item = impl Into<Uuid>>) -> Self {
        self.session_ids = Some(
            ids.into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    pub fn run_ids(mut self, ids: impl IntoIterator<Item = impl Into<Uuid>>) -> Self {
        self.run_ids = Some(
            ids.into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    pub fn reasons(
        mut self,
        reasons: impl IntoIterator<Item = impl Into<CheckpointReason>>,
    ) -> Self {
        self.reasons = Some(
            reasons
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
}

impl Default for FindCheckpointParams {
    fn default() -> Self {
        Self::new()
    }
}

impl From<FindCheckpointParams> for RepoFindCheckpointRecordParams {
    fn from(params: FindCheckpointParams) -> Self {
        Self {
            per_page: params.per_page,
            page: params.page,
            tenant_ids: params.tenant_ids,
            ids: params.ids,
            session_ids: params.session_ids,
            run_ids: params.run_ids,
            reasons: params.reasons,
            statuses: params.statuses,
            created_before: params.created_before,
            created_after: params.created_after,
        }
    }
}
