// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

use agentc_agent::graph::checkpoint::{
    CheckpointError, CheckpointReason as AgentCheckpointReason, CheckpointSnapshot,
    CheckpointSnapshotStore, RunStatus, SessionStore, StateStore,
};
use agentc_database::connection::ConnectionContext;
use agentc_domain::{
    repository::{
        checkpoint_record::{
            params::FindCheckpointRecordParams, traits::CheckpointRecordRepository,
        },
        run::{
            params::{Comparison, UpdateRunParams, UpdateRunParamsCondition, UpdateRunParamsSet},
            traits::RunRepository,
        },
        session::traits::SessionRepository,
    },
    types::{
        CheckpointReason as DomainCheckpointReason, CheckpointRecord, Run,
        RunStatus as DomainRunStatus, Session,
    },
};
use agentc_domain_sql::repository::{
    checkpoint_record::SqlCheckpointRecordRepository, run::SqlRunRepository,
    session::SqlSessionRepository,
};

use crate::{
    graph::ReActState,
    repository::{
        message::{
            params::FindMessageParams, sql::SqlMessageRepository, traits::MessageRepository,
        },
        state_snapshot::{
            params::FindStateSnapshotParams, sql::SqlStateSnapshotRepository,
            traits::StateSnapshotRepository,
        },
    },
    types::state_snapshot::StateSnapshot,
};

pub struct SqlReActSessionStore<'a> {
    session_repo: SqlSessionRepository<'a>,
    run_repo: SqlRunRepository<'a>,
}

impl<'a> SqlReActSessionStore<'a> {
    pub fn new(ctx: ConnectionContext<'a>) -> Self {
        Self {
            session_repo: SqlSessionRepository::new(ctx),
            run_repo: SqlRunRepository::new(ctx),
        }
    }
}

#[async_trait]
impl<'a> SessionStore for SqlReActSessionStore<'a> {
    type Error = CheckpointError;

    async fn save_session(&self, tenant_id: &str, session_id: Uuid) -> Result<(), Self::Error> {
        self.session_repo
            .save(vec![Session {
                id: session_id,
                tenant_id: tenant_id.to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }])
            .await
            .map_err(|e| CheckpointError::session_store_error(e.to_string()))?;

        Ok(())
    }

    async fn save_run(
        &self,
        tenant_id: &str,
        session_id: Uuid,
        run_id: Uuid,
    ) -> Result<(), Self::Error> {
        self.run_repo
            .save(vec![Run {
                id: run_id,
                tenant_id: tenant_id.to_string(),
                session_id,
                status: DomainRunStatus::Running,
                current_node: None,
                latest_checkpoint_id: None,
                last_interrupted_checkpoint_id: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }])
            .await
            .map_err(|e| CheckpointError::session_store_error(e.to_string()))?;

        Ok(())
    }

    async fn update_run_status(
        &self,
        tenant_id: &str,
        run_id: Uuid,
        status: RunStatus,
    ) -> Result<(), Self::Error> {
        self.run_repo
            .update(
                UpdateRunParams::new(tenant_id, run_id)
                    .set(UpdateRunParamsSet::Status(match status {
                        RunStatus::Running => DomainRunStatus::Running,
                        RunStatus::Interrupted => DomainRunStatus::Interrupted,
                        RunStatus::Failed => DomainRunStatus::Failed,
                        RunStatus::Completed => DomainRunStatus::Completed,
                        RunStatus::Cancelled => DomainRunStatus::Cancelled,
                    }))
                    .condition(UpdateRunParamsCondition::Status(Comparison::NotEq(
                        DomainRunStatus::Cancelled,
                    ))),
            )
            .await
            .map_err(|e| CheckpointError::session_store_error(e.to_string()))?;

        Ok(())
    }
}

pub struct SqlReActSnapshotStore<'a> {
    checkpoint_record_repo: SqlCheckpointRecordRepository<'a>,
}

impl<'a> SqlReActSnapshotStore<'a> {
    pub fn new(ctx: ConnectionContext<'a>) -> Self {
        Self {
            checkpoint_record_repo: SqlCheckpointRecordRepository::new(ctx),
        }
    }
}

#[async_trait]
impl<'a> CheckpointSnapshotStore for SqlReActSnapshotStore<'a> {
    type Error = CheckpointError;

    async fn save_snapshot(
        &self,
        snapshot: CheckpointSnapshot,
    ) -> Result<CheckpointSnapshot, Self::Error> {
        self.checkpoint_record_repo
            .save(vec![CheckpointRecord {
                id: snapshot.checkpoint_id,
                tenant_id: snapshot.tenant_id,
                session_id: snapshot.session_id,
                run_id: snapshot.run_id,
                node: snapshot.node,
                status: match snapshot.status {
                    RunStatus::Running => DomainRunStatus::Running,
                    RunStatus::Interrupted => DomainRunStatus::Interrupted,
                    RunStatus::Failed => DomainRunStatus::Failed,
                    RunStatus::Completed => DomainRunStatus::Completed,
                    RunStatus::Cancelled => DomainRunStatus::Cancelled,
                },
                reason: match snapshot.reason {
                    AgentCheckpointReason::Input => DomainCheckpointReason::Input,
                    AgentCheckpointReason::Step => DomainCheckpointReason::Step,
                    AgentCheckpointReason::Interrupt => DomainCheckpointReason::Interrupt,
                    AgentCheckpointReason::Finish => DomainCheckpointReason::Finish,
                },
                created_at: snapshot.created_at,
                parent_checkpoint_id: snapshot.parent_checkpoint_id,
                metadata: snapshot.metadata,
            }])
            .await
            .map_err(|e| CheckpointError::checkpoint_store_error(e.to_string()))
            .and_then(|mut saved| {
                saved
                    .pop()
                    .map(|r| CheckpointSnapshot {
                        checkpoint_id: r.id,
                        tenant_id: r.tenant_id,
                        session_id: r.session_id,
                        run_id: r.run_id,
                        node: r.node,
                        status: match r.status {
                            DomainRunStatus::Running => RunStatus::Running,
                            DomainRunStatus::Interrupted => RunStatus::Interrupted,
                            DomainRunStatus::Failed => RunStatus::Failed,
                            DomainRunStatus::Completed => RunStatus::Completed,
                            DomainRunStatus::Cancelled => RunStatus::Cancelled,
                        },
                        reason: match r.reason {
                            DomainCheckpointReason::Input => AgentCheckpointReason::Input,
                            DomainCheckpointReason::Step => AgentCheckpointReason::Step,
                            DomainCheckpointReason::Interrupt => AgentCheckpointReason::Interrupt,
                            DomainCheckpointReason::Finish => AgentCheckpointReason::Finish,
                        },
                        created_at: r.created_at,
                        parent_checkpoint_id: r.parent_checkpoint_id,
                        metadata: r.metadata,
                    })
                    .ok_or_else(|| {
                        CheckpointError::checkpoint_store_error(
                            "checkpoint record save returned no rows",
                        )
                    })
            })
    }

    async fn load_snapshot(
        &self,
        tenant_id: &str,
        checkpoint_id: Uuid,
    ) -> Result<Option<CheckpointSnapshot>, Self::Error> {
        self.checkpoint_record_repo
            .get(tenant_id, checkpoint_id)
            .await
            .map_err(|e| CheckpointError::checkpoint_store_error(e.to_string()))
            .map(|opt| {
                opt.map(|r| CheckpointSnapshot {
                    checkpoint_id: r.id,
                    tenant_id: r.tenant_id,
                    session_id: r.session_id,
                    run_id: r.run_id,
                    node: r.node,
                    status: match r.status {
                        DomainRunStatus::Running => RunStatus::Running,
                        DomainRunStatus::Interrupted => RunStatus::Interrupted,
                        DomainRunStatus::Failed => RunStatus::Failed,
                        DomainRunStatus::Completed => RunStatus::Completed,
                        DomainRunStatus::Cancelled => RunStatus::Cancelled,
                    },
                    reason: match r.reason {
                        DomainCheckpointReason::Input => AgentCheckpointReason::Input,
                        DomainCheckpointReason::Step => AgentCheckpointReason::Step,
                        DomainCheckpointReason::Interrupt => AgentCheckpointReason::Interrupt,
                        DomainCheckpointReason::Finish => AgentCheckpointReason::Finish,
                    },
                    created_at: r.created_at,
                    parent_checkpoint_id: r.parent_checkpoint_id,
                    metadata: r.metadata,
                })
            })
    }

    async fn load_latest_for_session(
        &self,
        tenant_id: &str,
        session_id: Uuid,
    ) -> Result<Option<CheckpointSnapshot>, Self::Error> {
        self.checkpoint_record_repo
            .find(
                FindCheckpointRecordParams::new()
                    .tenant_ids([tenant_id])
                    .session_ids([session_id])
                    .per_page(1u64),
            )
            .await
            .map_err(|e| CheckpointError::checkpoint_store_error(e.to_string()))
            .map(|page| {
                page.into_iter()
                    .next()
                    .map(|r| CheckpointSnapshot {
                        checkpoint_id: r.id,
                        tenant_id: r.tenant_id,
                        session_id: r.session_id,
                        run_id: r.run_id,
                        node: r.node,
                        status: match r.status {
                            DomainRunStatus::Running => RunStatus::Running,
                            DomainRunStatus::Interrupted => RunStatus::Interrupted,
                            DomainRunStatus::Failed => RunStatus::Failed,
                            DomainRunStatus::Completed => RunStatus::Completed,
                            DomainRunStatus::Cancelled => RunStatus::Cancelled,
                        },
                        reason: match r.reason {
                            DomainCheckpointReason::Input => AgentCheckpointReason::Input,
                            DomainCheckpointReason::Step => AgentCheckpointReason::Step,
                            DomainCheckpointReason::Interrupt => AgentCheckpointReason::Interrupt,
                            DomainCheckpointReason::Finish => AgentCheckpointReason::Finish,
                        },
                        created_at: r.created_at,
                        parent_checkpoint_id: r.parent_checkpoint_id,
                        metadata: r.metadata,
                    })
            })
    }
}

pub struct SqlReActStateStore<'a> {
    message_repo: SqlMessageRepository<'a>,
    state_snapshot_repo: SqlStateSnapshotRepository<'a>,
    checkpoint_record_repo: SqlCheckpointRecordRepository<'a>,
}

impl<'a> SqlReActStateStore<'a> {
    pub fn new(ctx: ConnectionContext<'a>) -> Self {
        Self {
            message_repo: SqlMessageRepository::new(ctx),
            state_snapshot_repo: SqlStateSnapshotRepository::new(ctx),
            checkpoint_record_repo: SqlCheckpointRecordRepository::new(ctx),
        }
    }
}

#[async_trait]
impl<'a> StateStore<ReActState> for SqlReActStateStore<'a> {
    type Error = CheckpointError;

    async fn save(
        &self,
        tenant_id: &str,
        session_id: Uuid,
        run_id: Uuid,
        checkpoint_id: Uuid,
        state: ReActState,
    ) -> Result<ReActState, Self::Error> {
        self.message_repo
            .save(
                state
                    .messages
                    .iter()
                    .cloned()
                    .map(|message| message.with_checkpoint_id(checkpoint_id))
                    .collect(),
            )
            .await
            .map_err(|e| CheckpointError::state_store_error(e.to_string()))?;

        let existing = self
            .state_snapshot_repo
            .find(
                FindStateSnapshotParams::new()
                    .tenant_ids([tenant_id])
                    .session_ids([session_id])
                    .run_ids([run_id])
                    .checkpoint_ids([checkpoint_id])
                    .per_page(1u64),
            )
            .await
            .map_err(|e| CheckpointError::state_store_error(e.to_string()))?
            .into_iter()
            .next();

        self.state_snapshot_repo
            .save(vec![StateSnapshot {
                id: existing
                    .as_ref()
                    .map(|s| s.id)
                    .unwrap_or_else(Uuid::new_v4),
                tenant_id: tenant_id.to_string(),
                session_id,
                run_id,
                checkpoint_id: Some(checkpoint_id),
                model: state.model.clone(),
                capability_override: state.capability_override.clone(),
                tools: if state.tools.is_empty() {
                    None
                } else {
                    Some(state.tools.clone())
                },
                context_vars: if state.context_vars.is_empty() {
                    None
                } else {
                    Some(state.context_vars.clone())
                },
                context: state.context.clone(),
                created_at: existing
                    .as_ref()
                    .map(|s| s.created_at)
                    .unwrap_or_else(Utc::now),
                updated_at: Utc::now(),
            }])
            .await
            .map_err(|e| CheckpointError::state_store_error(e.to_string()))?;

        Ok(state)
    }

    async fn load(
        &self,
        tenant_id: &str,
        session_id: Uuid,
        run_id: Uuid,
        checkpoint_id: Uuid,
    ) -> Result<Option<ReActState>, Self::Error> {
        let snapshot = match self
            .state_snapshot_repo
            .find(
                FindStateSnapshotParams::new()
                    .tenant_ids([tenant_id])
                    .session_ids([session_id])
                    .checkpoint_ids([checkpoint_id])
                    .per_page(1u64),
            )
            .await
            .map_err(|e| CheckpointError::state_store_error(e.to_string()))?
            .into_iter()
            .next()
        {
            Some(s) => s,
            None => return Ok(None),
        };

        let ancestry = self
            .checkpoint_record_repo
            .ancestry(tenant_id, checkpoint_id)
            .await
            .map_err(|e| CheckpointError::state_store_error(e.to_string()))?;

        let order = ancestry
            .iter()
            .enumerate()
            .map(|(index, record)| (record.id, index))
            .collect::<HashMap<_, _>>();

        let mut messages = self
            .message_repo
            .find(
                FindMessageParams::new()
                    .tenant_ids([tenant_id])
                    .session_ids([session_id])
                    .checkpoint_ids(ancestry.iter().map(|record| record.id))
                    .no_limit(),
            )
            .await
            .map_err(|e| CheckpointError::state_store_error(e.to_string()))?
            .into_iter()
            .collect::<Vec<_>>();

        // Ancestry order first (legacy NULL stamps sort ahead as base history), then
        // creation order within a checkpoint.
        messages.sort_by_key(|message| {
            (
                message
                    .checkpoint_id()
                    .and_then(|id| order.get(id).copied())
                    .unwrap_or(0),
                *message.created_at(),
            )
        });

        Ok(Some(ReActState {
            run_id,
            session_id,
            model: snapshot.model,
            capability_override: snapshot.capability_override,
            tools: snapshot.tools.unwrap_or_default(),
            context_vars: snapshot
                .context_vars
                .unwrap_or_default(),
            messages,
            context: snapshot.context,
        }))
    }
}
