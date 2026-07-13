// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use std::sync::LazyLock;
use uuid::Uuid;

use agentc_agent::types::params::RunParams as AgentRunParams;
use agentc_domain::{
    repository::{
        run::{
            params::DeleteRunParams,
            traits::{RunRepoProvider, RunRepository},
        },
        scope::RepoScopeFactory,
    },
    types::Page,
};
use agentc_telemetry::{
    Level, error, info, instrument,
    metrics::{UpDownCounter, meter},
};

use crate::service::{
    application::ApplicationService,
    errors::ServiceError,
    types::run::{FindRunParams, RunEvent, RunParams, RunResponse, RunStream},
};

static RUN_IN_FLIGHT: LazyLock<UpDownCounter<i64>> = LazyLock::new(|| {
    meter("agentc-agent-react")
        .i64_up_down_counter("agent.react.run.in_flight")
        .with_unit("{run}")
        .with_description("Number of in-flight react agent runs.")
        .build()
});

/// Tracks a single in-flight run for the `agent.react.run.in_flight` gauge.
struct RunInFlightGuard {
    active: bool,
}

impl RunInFlightGuard {
    fn new() -> Self {
        RUN_IN_FLIGHT.add(1, &[]);
        Self { active: true }
    }

    fn complete(&mut self) {
        if self.active {
            self.active = false;
            RUN_IN_FLIGHT.add(-1, &[]);
        }
    }
}

impl Drop for RunInFlightGuard {
    fn drop(&mut self) {
        self.complete();
    }
}

#[async_trait]
pub trait RunOperations: Send + Sync {
    async fn get_run(&self, tenant_id: &str, id: Uuid) -> Result<RunResponse, ServiceError>;
    async fn find_runs(&self, params: FindRunParams) -> Result<Page<RunResponse>, ServiceError>;
    async fn delete_runs(&self, tenant_id: &str, ids: &[Uuid]) -> Result<(), ServiceError>;
    async fn run(&self, params: RunParams) -> Result<BoxStream<'static, RunEvent>, ServiceError>;
    async fn cancel_run(&self, tenant_id: &str, run_id: Uuid) -> Result<bool, ServiceError>;
}

#[async_trait]
impl RunOperations for ApplicationService {
    #[instrument(
        level = Level::INFO,
        skip(self, tenant_id, id),
        fields(
            tenant_id = tenant_id,
            run_id = ?id,
        )
    )]
    async fn get_run(&self, tenant_id: &str, id: Uuid) -> Result<RunResponse, ServiceError> {
        let tenant_id = tenant_id.to_string();

        self.scope_factory
            .ro_scope(|scope| {
                Box::pin(async move {
                    scope
                        .run_repo()
                        .get(&tenant_id, id)
                        .await?
                        .map(|run| RunResponse::from_entity(&run))
                        .ok_or_else(|| ServiceError::run_not_found(id))
                })
            })
            .await
    }

    #[instrument(
        level = Level::INFO,
        skip(self, params),
        fields(
            per_page = &params.per_page,
            page = &params.page,
            tenant_ids = ?params.tenant_ids,
            session_ids = ?params.session_ids,
        )
    )]
    async fn find_runs(&self, params: FindRunParams) -> Result<Page<RunResponse>, ServiceError> {
        self.scope_factory
            .ro_scope(|scope| {
                Box::pin(async move {
                    Ok(scope
                        .run_repo()
                        .find(params.into())
                        .await?
                        .map(|run| RunResponse::from_entity(&run)))
                })
            })
            .await
    }

    #[instrument(
        level = Level::INFO,
        skip(self, tenant_id, ids),
        fields(
            tenant_id = tenant_id,
            run_ids = ?ids,
        )
    )]
    async fn delete_runs(&self, tenant_id: &str, ids: &[Uuid]) -> Result<(), ServiceError> {
        let tenant_id = tenant_id.to_string();
        let ids = ids.to_vec();

        self.scope_factory
            .rw_scope(|scope| {
                Box::pin(async move {
                    // TODO: Emit run deleted events in outbox for each run
                    scope
                        .run_repo()
                        .delete(DeleteRunParams {
                            tenant_id: tenant_id.clone(),
                            ids: ids.clone(),
                        })
                        .await?;

                    info!(
                        event = "DeletedRuns",
                        tenant_id = &tenant_id,
                        run_ids = ?ids,
                    );

                    Ok(())
                })
            })
            .await
    }

    #[instrument(
        level = Level::INFO,
        skip(self, params),
        fields(
            tenant_id = &params.tenant_id,
            session_id = ?params.session_id,
            run_id = ?params.run_id,
        )
    )]
    async fn run(&self, params: RunParams) -> Result<BoxStream<'static, RunEvent>, ServiceError> {
        self.agent
            .run(
                AgentRunParams::new(params.to_input(), params.tenant_id.clone())
                    .with_session_id(params.session_id)
                    .with_run_id(params.run_id),
            )
            .await
            .map_err(ServiceError::from)
            .map(move |stream| {
                let mut in_flight = RunInFlightGuard::new();

                RunStream::new(stream)
                    .inspect(move |event| match event {
                        RunEvent::RunStarted { session_id, run_id, .. } => info!(
                            event = "RunStarted",
                            tenant_id = &params.tenant_id,
                            session_id = ?session_id,
                            run_id = ?run_id,
                        ),
                        RunEvent::RunFinished { session_id, run_id, .. } => {
                            in_flight.complete();

                            info!(
                                event = "RunFinished",
                                tenant_id = &params.tenant_id,
                                session_id = ?session_id,
                                run_id = ?run_id,
                            );
                        }
                        RunEvent::RunError { session_id, run_id, error, code, .. } => {
                            in_flight.complete();

                            error!(
                                event = "RunError",
                                tenant_id = &params.tenant_id,
                                session_id = ?session_id,
                                run_id = ?run_id,
                                error = ?error,
                                code = code.as_deref().unwrap_or("none"),
                            );
                        }
                        _ => {}
                    })
                    .boxed()
            })
    }

    #[instrument(
        level = Level::INFO,
        skip(self, tenant_id, run_id),
        fields(
            tenant_id = tenant_id,
            run_id = ?run_id,
        )
    )]
    async fn cancel_run(&self, tenant_id: &str, run_id: Uuid) -> Result<bool, ServiceError> {
        self.agent
            .cancel(tenant_id, run_id)
            .await
            .map_err(ServiceError::from)
    }
}
