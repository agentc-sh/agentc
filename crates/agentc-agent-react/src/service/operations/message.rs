// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use uuid::Uuid;

use agentc_domain::{repository::scope::RepoScopeFactory, types::Page};
use agentc_telemetry::{Level, info, instrument};

use crate::{
    repository::message::{
        params::DeleteMessageParams,
        traits::{MessageRepoProvider, MessageRepository},
    },
    service::{
        application::ApplicationService,
        errors::ServiceError,
        types::message::{CreateMessageParams, FindMessageParams, MessageResponse},
    },
};

#[async_trait]
pub trait MessageOperations: Send + Sync {
    async fn create_message(
        &self,
        tenant_id: &str,
        session_id: Uuid,
        params: CreateMessageParams,
    ) -> Result<MessageResponse, ServiceError>;
    async fn get_message(&self, tenant_id: &str, id: Uuid)
    -> Result<MessageResponse, ServiceError>;
    async fn find_messages(
        &self,
        params: FindMessageParams,
    ) -> Result<Page<MessageResponse>, ServiceError>;
    async fn delete_messages(&self, tenant_id: &str, ids: &[Uuid]) -> Result<(), ServiceError>;
}

#[async_trait]
impl MessageOperations for ApplicationService {
    #[instrument(
        level = Level::TRACE,
        skip(self, tenant_id, session_id, params),
        fields(
            tenant_id = &tenant_id,
            session_id = ?session_id,
            message_id = ?params.id(),
        )
    )]
    async fn create_message(
        &self,
        tenant_id: &str,
        session_id: Uuid,
        params: CreateMessageParams,
    ) -> Result<MessageResponse, ServiceError> {
        let tenant_id = tenant_id.to_string();

        self.scope_factory
            .rw_scope(|scope| {
                Box::pin(async move {
                    if let Some(existing) = scope
                        .message_repo()
                        .get(&tenant_id, *params.id())
                        .await?
                    {
                        return Err(ServiceError::message_already_exists(*existing.id()));
                    }

                    let message = scope
                        .message_repo()
                        .save(vec![params.to_entity(tenant_id, session_id)])
                        .await?
                        .into_iter()
                        .next()
                        .expect("expected one message to be saved");

                    // TODO: Emit message created event in outbox

                    info!(
                        event = "CreatedMessage",
                        tenant_id = message.tenant_id(),
                        session_id = ?message.session_id(),
                        message_id = ?message.id(),
                    );

                    Ok(MessageResponse::from_entity(&message))
                })
            })
            .await
    }

    #[instrument(
        level = Level::TRACE,
        skip(self, tenant_id, id),
        fields(
            tenant_id = tenant_id,
            message_id = ?id,
        )
    )]
    async fn get_message(
        &self,
        tenant_id: &str,
        id: Uuid,
    ) -> Result<MessageResponse, ServiceError> {
        let tenant_id = tenant_id.to_string();

        self.scope_factory
            .ro_scope(|scope| {
                Box::pin(async move {
                    scope
                        .message_repo()
                        .get(&tenant_id, id)
                        .await?
                        .map(|message| MessageResponse::from_entity(&message))
                        .ok_or_else(|| ServiceError::message_not_found(id))
                })
            })
            .await
    }

    #[instrument(
        level = Level::TRACE,
        skip(self, params),
         fields(
            per_page = &params.per_page,
            page = &params.page,
            tenant_ids = ?params.tenant_ids,
            session_ids = ?params.session_ids,
        )
    )]
    async fn find_messages(
        &self,
        params: FindMessageParams,
    ) -> Result<Page<MessageResponse>, ServiceError> {
        self.scope_factory
            .ro_scope(|scope| {
                Box::pin(async move {
                    Ok(scope
                        .message_repo()
                        .find(params.into())
                        .await?
                        .map(|message| MessageResponse::from_entity(&message)))
                })
            })
            .await
    }

    #[instrument(
        level = Level::TRACE,
        skip(self, tenant_id, ids),
        fields(
            tenant_id = tenant_id,
            message_ids = ?ids,
        )
    )]
    async fn delete_messages(&self, tenant_id: &str, ids: &[Uuid]) -> Result<(), ServiceError> {
        let tenant_id = tenant_id.to_string();
        let ids = ids.to_vec();

        self.scope_factory
            .rw_scope(|scope| {
                Box::pin(async move {
                    scope
                        .message_repo()
                        .delete(DeleteMessageParams {
                            tenant_id: tenant_id.clone(),
                            ids: ids.clone(),
                        })
                        .await?;

                    info!(
                        event = "DeletedMessages",
                        tenant_id = ?tenant_id,
                        message_ids = ?ids,
                    );

                    Ok(())
                })
            })
            .await
    }
}
