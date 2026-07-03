// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use futures::future::BoxFuture;
use std::sync::Arc;

use agentc_agent::graph::checkpoint::{
    CheckpointError, CheckpointStoreContext, CheckpointStoreHandle,
};
use agentc_database::{connection::ConnectionContext, database::Database, orm::TransactionTrait};

use crate::{
    checkpoint::store::{SqlReActSessionStore, SqlReActSnapshotStore, SqlReActStateStore},
    graph::runtime::ReActNode,
};

pub struct SqlReActCheckpointStoreContext<'a> {
    ctx: ConnectionContext<'a>,
}

impl<'a> SqlReActCheckpointStoreContext<'a> {
    pub fn new(ctx: ConnectionContext<'a>) -> Self {
        Self { ctx }
    }
}

impl<'a> CheckpointStoreContext<ReActNode> for SqlReActCheckpointStoreContext<'a> {
    type SessionStore = SqlReActSessionStore<'a>;
    type SnapshotStore = SqlReActSnapshotStore<'a>;
    type StateStore = SqlReActStateStore<'a>;

    fn session_store(&self) -> Self::SessionStore {
        SqlReActSessionStore::new(self.ctx)
    }

    fn snapshot_store(&self) -> Self::SnapshotStore {
        SqlReActSnapshotStore::new(self.ctx)
    }

    fn state_store(&self) -> Self::StateStore {
        SqlReActStateStore::new(self.ctx)
    }
}

pub struct SqlReActCheckpointStoreHandle {
    database: Arc<Database>,
}

impl SqlReActCheckpointStoreHandle {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

#[async_trait]
impl CheckpointStoreHandle<ReActNode> for SqlReActCheckpointStoreHandle {
    type Context<'a>
        = SqlReActCheckpointStoreContext<'a>
    where
        Self: 'a;

    async fn run<F, R>(&self, f: F) -> Result<R, CheckpointError>
    where
        F: for<'a> FnOnce(&'a Self::Context<'a>) -> BoxFuture<'a, Result<R, CheckpointError>>
            + Send,
        R: Send,
    {
        let txn = self
            .database
            .get_write_connection()
            .begin()
            .await
            .map_err(|e| CheckpointError::unexpected_error(e.to_string()))?;

        match f(&SqlReActCheckpointStoreContext::new(ConnectionContext::Transaction(&txn))).await {
            Ok(result) => {
                txn.commit()
                    .await
                    .map_err(|e| CheckpointError::unexpected_error(e.to_string()))?;

                Ok(result)
            }
            Err(err) => {
                txn.rollback()
                    .await
                    .map_err(|e| CheckpointError::unexpected_error(e.to_string()))?;

                Err(err)
            }
        }
    }
}
