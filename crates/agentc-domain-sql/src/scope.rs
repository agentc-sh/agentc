// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use futures::future::BoxFuture;
use std::sync::Arc;
use thiserror::Error;

use agentc_database::{
    connection::ConnectionContext,
    database::Database,
    errors::DatabaseError,
    orm::{DbErr, TransactionError, TransactionTrait},
};
use agentc_domain::repository::{
    checkpoint_record::{errors::CheckpointRecordRepoError, traits::CheckpointRecordRepoProvider},
    run::{errors::RunRepoError, traits::RunRepoProvider},
    scope::{RepoScope, RepoScopeFactory},
    session::{errors::SessionRepoError, traits::SessionRepoProvider},
};

use crate::repository::{
    checkpoint_record::SqlCheckpointRecordRepository, run::SqlRunRepository,
    session::SqlSessionRepository,
};

#[derive(Error, Debug)]
pub enum SqlScopeFactoryError {
    #[error("session repository error: {0}")]
    SessionRepo(#[from] SessionRepoError),

    #[error("run repository error: {0}")]
    RunRepo(#[from] RunRepoError),

    #[error("checkpoint record repository error: {0}")]
    CheckpointRecordRepo(#[from] CheckpointRecordRepoError),

    #[error("unexpected error: {message}")]
    Unexpected {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl SqlScopeFactoryError {
    pub fn source_unexpected(source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        let source = source.into();
        Self::Unexpected {
            message: source.to_string(),
            source: Some(source),
        }
    }
}

impl<T> From<TransactionError<T>> for SqlScopeFactoryError
where
    T: Into<SqlScopeFactoryError>,
{
    fn from(err: TransactionError<T>) -> Self {
        match err {
            TransactionError::Connection(db_err) => Self::source_unexpected(db_err),
            TransactionError::Transaction(err) => err.into(),
        }
    }
}

impl From<DbErr> for SqlScopeFactoryError {
    fn from(err: DbErr) -> Self {
        Self::source_unexpected(err)
    }
}

impl From<DatabaseError> for SqlScopeFactoryError {
    fn from(err: DatabaseError) -> Self {
        Self::source_unexpected(err)
    }
}

/// A SQL-based implementation of [`RepoScope`](agentc_domain::repository::scope::RepoScope) that provides repositories for sessions and runs.
pub struct SqlScope<'a> {
    ctx: ConnectionContext<'a>,
}

impl<'a> SqlScope<'a> {
    pub fn new(ctx: ConnectionContext<'a>) -> Self {
        Self { ctx }
    }

    pub fn ctx(&self) -> ConnectionContext<'a> {
        self.ctx
    }
}

impl RepoScope for SqlScope<'_> {}

impl SessionRepoProvider for SqlScope<'_> {
    type Repo<'a>
        = SqlSessionRepository<'a>
    where
        Self: 'a;

    fn session_repo(&self) -> Self::Repo<'_> {
        SqlSessionRepository::new(self.ctx)
    }
}

impl RunRepoProvider for SqlScope<'_> {
    type Repo<'a>
        = SqlRunRepository<'a>
    where
        Self: 'a;

    fn run_repo(&self) -> Self::Repo<'_> {
        SqlRunRepository::new(self.ctx)
    }
}

impl CheckpointRecordRepoProvider for SqlScope<'_> {
    type Repo<'a>
        = SqlCheckpointRecordRepository<'a>
    where
        Self: 'a;

    fn checkpoint_record_repo(&self) -> Self::Repo<'_> {
        SqlCheckpointRecordRepository::new(self.ctx)
    }
}

/// A factory for creating [`SqlScope`](crate::scope::SqlScope) instances, which manage database connections and transactions.
pub struct SqlScopeFactory {
    database: Arc<Database>,
}

impl SqlScopeFactory {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

#[async_trait]
impl RepoScopeFactory for SqlScopeFactory {
    type Scope<'scope>
        = SqlScope<'scope>
    where
        Self: 'scope;
    type Error = SqlScopeFactoryError;

    async fn rw_scope<F, R, E>(&self, f: F) -> Result<R, E>
    where
        E: From<Self::Error> + Send,
        F: for<'scope> FnOnce(&'scope mut Self::Scope<'scope>) -> BoxFuture<'scope, Result<R, E>>
            + Send,
        R: Send,
    {
        let txn = self
            .database
            .get_write_connection()
            .begin()
            .await
            .map_err(SqlScopeFactoryError::from)?;

        match f(&mut SqlScope::new(ConnectionContext::Transaction(&txn))).await {
            Ok(result) => {
                txn.commit()
                    .await
                    .map_err(SqlScopeFactoryError::from)?;

                Ok(result)
            }
            Err(err) => {
                txn.rollback()
                    .await
                    .map_err(SqlScopeFactoryError::from)?;

                Err(err)
            }
        }
    }

    async fn ro_scope<F, R, E>(&self, f: F) -> Result<R, E>
    where
        E: From<Self::Error> + Send,
        F: for<'scope> FnOnce(&'scope Self::Scope<'scope>) -> BoxFuture<'scope, Result<R, E>>
            + Send,
        R: Send,
    {
        f(&SqlScope::new(self.database.read_ctx())).await
    }
}
