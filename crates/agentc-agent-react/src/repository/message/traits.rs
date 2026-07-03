// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use uuid::Uuid;

use agentc_domain::types::Page;

use crate::{
    repository::message::{
        errors::MessageRepoError,
        params::{DeleteMessageParams, FindMessageParams},
    },
    types::message::Message,
};

#[async_trait]
pub trait MessageRepository: Send + Sync {
    async fn save(&self, messages: Vec<Message>) -> Result<Vec<Message>, MessageRepoError>;
    async fn get(&self, tenant_id: &str, id: Uuid) -> Result<Option<Message>, MessageRepoError>;
    async fn find(&self, params: FindMessageParams) -> Result<Page<Message>, MessageRepoError>;
    async fn delete(&self, params: DeleteMessageParams) -> Result<(), MessageRepoError>;
}

#[async_trait]
pub trait MessageRepoProvider {
    type Repo<'a>: MessageRepository + Send
    where
        Self: 'a;

    fn message_repo(&self) -> Self::Repo<'_>;
}
