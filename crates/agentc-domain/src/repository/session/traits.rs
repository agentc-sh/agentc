// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    repository::session::{
        errors::SessionRepoError,
        params::{DeleteSessionParams, FindSessionParams},
    },
    types::{Page, Session},
};

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn save(&self, sessions: Vec<Session>) -> Result<Vec<Session>, SessionRepoError>;
    async fn get(&self, tenant_id: &str, id: Uuid) -> Result<Option<Session>, SessionRepoError>;
    async fn find(&self, params: FindSessionParams) -> Result<Page<Session>, SessionRepoError>;
    async fn delete(&self, params: DeleteSessionParams) -> Result<(), SessionRepoError>;
}

#[async_trait]
pub trait SessionRepoProvider {
    type Repo<'a>: SessionRepository + Send
    where
        Self: 'a;

    fn session_repo(&self) -> Self::Repo<'_>;
}
