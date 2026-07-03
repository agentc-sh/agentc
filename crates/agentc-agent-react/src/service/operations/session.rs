// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use uuid::Uuid;

use agentc_domain::{
    repository::{
        scope::RepoScopeFactory,
        session::{
            params::DeleteSessionParams,
            traits::{SessionRepoProvider, SessionRepository},
        },
    },
    types::Page,
};
use agentc_telemetry::{Level, info, instrument};

use crate::service::{
    application::ApplicationService,
    errors::ServiceError,
    types::session::{CreateSessionParams, FindSessionParams, SessionResponse},
};

#[async_trait]
pub trait SessionOperations: Send + Sync {
    async fn create_session(
        &self,
        params: CreateSessionParams,
    ) -> Result<SessionResponse, ServiceError>;
    async fn get_session(&self, tenant_id: &str, id: Uuid)
    -> Result<SessionResponse, ServiceError>;
    async fn find_sessions(
        &self,
        params: FindSessionParams,
    ) -> Result<Page<SessionResponse>, ServiceError>;
    async fn delete_sessions(&self, tenant_id: &str, ids: &[Uuid]) -> Result<(), ServiceError>;
}

#[async_trait]
impl SessionOperations for ApplicationService {
    #[instrument(
        level = Level::TRACE,
        skip(self, params),
        fields(
            tenant_id = &params.tenant_id,
            session_id = ?params.id,
        )
    )]
    async fn create_session(
        &self,
        params: CreateSessionParams,
    ) -> Result<SessionResponse, ServiceError> {
        self.scope_factory
            .rw_scope(|scope| {
                Box::pin(async move {
                    if let Some(existing) = scope
                        .session_repo()
                        .get(&params.tenant_id, params.id)
                        .await?
                    {
                        return Err(ServiceError::session_already_exists(existing.id));
                    }

                    let session = scope
                        .session_repo()
                        .save(vec![params.to_entity()])
                        .await?
                        .into_iter()
                        .next()
                        .expect("expected one session to be saved");

                    // TODO: Emit session created event in outbox
                    info!(
                        event = "CreatedSession",
                        tenant_id = &session.tenant_id,
                        session_id = ?session.id,
                    );

                    Ok(SessionResponse::from_entity(&session))
                })
            })
            .await
    }

    #[instrument(
        level = Level::TRACE,
        skip(self, tenant_id, id),
        fields(
            tenant_id = tenant_id,
            session_id = ?id,
        )
    )]
    async fn get_session(
        &self,
        tenant_id: &str,
        id: Uuid,
    ) -> Result<SessionResponse, ServiceError> {
        let tenant_id = tenant_id.to_string();

        self.scope_factory
            .ro_scope(|scope| {
                Box::pin(async move {
                    scope
                        .session_repo()
                        .get(&tenant_id, id)
                        .await?
                        .map(|session| SessionResponse::from_entity(&session))
                        .ok_or_else(|| ServiceError::session_not_found(id))
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
        )
    )]
    async fn find_sessions(
        &self,
        params: FindSessionParams,
    ) -> Result<Page<SessionResponse>, ServiceError> {
        self.scope_factory
            .ro_scope(|scope| {
                Box::pin(async move {
                    Ok(scope
                        .session_repo()
                        .find(params.into())
                        .await?
                        .map(|session| SessionResponse::from_entity(&session)))
                })
            })
            .await
    }

    #[instrument(
        level = Level::TRACE,
        skip(self, tenant_id, ids),
        fields(
            tenant_id = tenant_id,
            session_ids = ?ids,
        )
    )]
    async fn delete_sessions(&self, tenant_id: &str, ids: &[Uuid]) -> Result<(), ServiceError> {
        let tenant_id = tenant_id.to_string();
        let ids = ids.to_vec();

        self.scope_factory
            .rw_scope(|scope| {
                Box::pin(async move {
                    // TODO: Emit session deleted events in outbox

                    scope
                        .session_repo()
                        .delete(DeleteSessionParams {
                            tenant_id: tenant_id.clone(),
                            ids: ids.clone(),
                        })
                        .await?;

                    info!(
                        event = "DeletedSessions",
                        tenant_id = &tenant_id,
                        session_ids = ?ids,
                    );

                    Ok(())
                })
            })
            .await
    }
}
