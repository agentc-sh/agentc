// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use futures::future::try_join_all;
use serde::{Deserialize, Serialize};
use std::{
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use sea_orm_migration::MigratorTrait;

use crate::{
    connection::ConnectionContext,
    coordinator::{
        MigrationCoordinator, NoopCoordinator, PostgresAdvisoryLockCoordinator,
    },
    errors::DatabaseError,
    orm::{
        ConnectOptions, ConnectionTrait, Database as SeaORMDatabase, DatabaseBackend,
        DatabaseConnection, DatabaseTransaction, TransactionError, TransactionTrait,
    },
};

#[derive(Debug)]
pub struct Database {
    primary: DatabaseConnection,
    replicas: Vec<DatabaseConnection>,
    replica_index: AtomicUsize,
}

impl Database {
    pub fn new(primary: DatabaseConnection, replicas: Vec<DatabaseConnection>) -> Self {
        Self {
            primary,
            replicas,
            replica_index: AtomicUsize::new(0),
        }
    }

    pub fn builder() -> DatabaseBuilder {
        DatabaseBuilder::new()
    }

    pub fn backend(&self) -> DatabaseBackend {
        self.primary.get_database_backend()
    }

    pub fn get_write_connection(&self) -> &DatabaseConnection {
        &self.primary
    }

    pub fn get_read_connection(&self) -> &DatabaseConnection {
        if self.replicas.is_empty() {
            return self.get_write_connection();
        }

        &self.replicas[self
            .replica_index
            .fetch_add(1, Ordering::SeqCst)
            % self.replicas.len()]
    }

    pub fn write_ctx(&self) -> ConnectionContext<'_> {
        ConnectionContext::Connection(self.get_write_connection())
    }

    pub fn read_ctx(&self) -> ConnectionContext<'_> {
        ConnectionContext::Connection(self.get_read_connection())
    }

    pub async fn close(self) -> Result<(), DatabaseError> {
        self.primary
            .close()
            .await
            .map_err(DatabaseError::from)?;

        for replica in self.replicas {
            replica
                .close()
                .await
                .map_err(DatabaseError::from)?;
        }

        Ok(())
    }

    pub async fn run_migrations<M: MigratorTrait>(&self) -> Result<(), DatabaseError> {
        match self.backend() {
            DatabaseBackend::Postgres => {
                PostgresAdvisoryLockCoordinator
                    .run::<M>(&self.primary)
                    .await
            }
            _ => NoopCoordinator.run::<M>(&self.primary).await,
        }
    }

    pub async fn rw_transaction<F, T>(&self, f: F) -> Result<T, DatabaseError>
    where
        F: for<'b> FnOnce(
                &'b DatabaseTransaction,
            )
                -> Pin<Box<dyn Future<Output = Result<T, DatabaseError>> + Send + 'b>>
            + Send,
        T: Send,
    {
        match self
            .get_write_connection()
            .transaction(f)
            .await
        {
            Ok(result) => Ok(result),
            Err(e) => match e {
                TransactionError::Connection(conn_err) => {
                    Err(DatabaseError::DatabaseError(conn_err))
                }
                TransactionError::Transaction(txn_err) => Err(txn_err),
            },
        }
    }

    pub async fn ro_transaction<F, T>(&self, f: F) -> Result<T, DatabaseError>
    where
        F: for<'b> FnOnce(
                &'b DatabaseTransaction,
            )
                -> Pin<Box<dyn Future<Output = Result<T, DatabaseError>> + Send + 'b>>
            + Send,
        T: Send,
    {
        match self
            .get_read_connection()
            .transaction(f)
            .await
        {
            Ok(result) => Ok(result),
            Err(e) => match e {
                TransactionError::Connection(conn_err) => {
                    Err(DatabaseError::DatabaseError(conn_err))
                }
                TransactionError::Transaction(txn_err) => Err(txn_err),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseOptions {
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
    pub connect_timeout: Option<Duration>,
    pub idle_timeout: Option<Duration>,
    pub acquire_timeout: Option<Duration>,
    pub max_lifetime: Option<Duration>,
    pub enable_logging: bool,
}

impl DatabaseOptions {
    pub fn new() -> Self {
        Self {
            max_connections: None,
            min_connections: None,
            connect_timeout: None,
            idle_timeout: None,
            acquire_timeout: None,
            max_lifetime: None,
            enable_logging: false,
        }
    }

    pub fn with_max_connections(mut self, max: impl Into<u32>) -> Self {
        self.max_connections = Some(max.into());
        self
    }

    pub fn with_min_connections(mut self, min: impl Into<u32>) -> Self {
        self.min_connections = Some(min.into());
        self
    }

    pub fn with_connect_timeout(mut self, timeout: impl Into<Duration>) -> Self {
        self.connect_timeout = Some(timeout.into());
        self
    }

    pub fn with_idle_timeout(mut self, timeout: impl Into<Duration>) -> Self {
        self.idle_timeout = Some(timeout.into());
        self
    }

    pub fn with_acquire_timeout(mut self, timeout: impl Into<Duration>) -> Self {
        self.acquire_timeout = Some(timeout.into());
        self
    }

    pub fn with_max_lifetime(mut self, lifetime: impl Into<Duration>) -> Self {
        self.max_lifetime = Some(lifetime.into());
        self
    }

    pub fn with_logging(mut self, enable: bool) -> Self {
        self.enable_logging = enable;
        self
    }

    pub(crate) fn build(self, dsn: &str) -> ConnectOptions {
        let mut options = ConnectOptions::new(dsn);

        if let Some(max) = self.max_connections {
            options.max_connections(max);
        }
        if let Some(min) = self.min_connections {
            options.min_connections(min);
        }
        if let Some(timeout) = self.connect_timeout {
            options.connect_timeout(timeout);
        }
        if let Some(timeout) = self.idle_timeout {
            options.idle_timeout(timeout);
        }
        if let Some(timeout) = self.acquire_timeout {
            options.acquire_timeout(timeout);
        }
        if let Some(lifetime) = self.max_lifetime {
            options.max_lifetime(lifetime);
        }

        options.sqlx_logging(self.enable_logging);

        options
    }
}

impl Default for DatabaseOptions {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DatabaseBuilder {
    primary: Option<String>,
    replicas: Option<Vec<String>>,
    options: Option<DatabaseOptions>,
}

impl Default for DatabaseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DatabaseBuilder {
    pub fn new() -> Self {
        Self {
            primary: None,
            replicas: None,
            options: None,
        }
    }

    pub fn with_primary(mut self, primary: impl Into<String>) -> Self {
        self.primary = Some(primary.into());
        self
    }

    pub fn with_replicas(mut self, replicas: Vec<impl Into<String>>) -> Self {
        if replicas.is_empty() {
            return self;
        }

        self.replicas = Some(
            replicas
                .into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    pub fn with_options(mut self, options: DatabaseOptions) -> Self {
        self.options = Some(options);
        self
    }

    pub async fn build(self) -> Result<Database, DatabaseError> {
        let primary = self
            .primary
            .ok_or(DatabaseError::MissingPrimaryDsn)?;
        let replicas = self.replicas.unwrap_or_default();
        let options = self.options.unwrap_or_default();

        let database = Database::new(
            SeaORMDatabase::connect(options.clone().build(&primary))
                .await
                .map_err(DatabaseError::from)?,
            try_join_all(
                replicas
                    .into_iter()
                    .map(|dsn| SeaORMDatabase::connect(options.clone().build(&dsn)))
                    .collect::<Vec<_>>(),
            )
            .await
            .map_err(DatabaseError::from)?,
        );

        Ok(database)
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use sea_orm_migration::prelude::*;

    use crate::{
        database::Database,
        orm::{ConnectOptions, Database as SeaORMDatabase},
    };

    #[derive(DeriveMigrationName)]
    struct CreateWidget;

    #[async_trait]
    impl MigrationTrait for CreateWidget {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(Widget::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Widget::Id)
                                .integer()
                                .not_null()
                                .primary_key(),
                        )
                        .to_owned(),
                )
                .await
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(
                    Table::drop()
                        .table(Widget::Table)
                        .to_owned(),
                )
                .await
        }
    }

    #[derive(DeriveIden)]
    enum Widget {
        Table,
        Id,
    }

    struct TestMigrator;

    impl MigratorTrait for TestMigrator {
        fn migrations() -> Vec<Box<dyn MigrationTrait>> {
            vec![Box::new(CreateWidget)]
        }
    }

    /// A single-connection in-memory pool keeps one shared database alive across
    /// both runs, so the second run exercises the already-applied path rather
    /// than a fresh empty database.
    #[tokio::test]
    async fn run_migrations_is_idempotent_on_sqlite() {
        let mut options = ConnectOptions::new("sqlite::memory:");
        options.max_connections(1);

        let database = Database::new(
            SeaORMDatabase::connect(options)
                .await
                .unwrap(),
            vec![],
        );

        database
            .run_migrations::<TestMigrator>()
            .await
            .unwrap();

        database
            .run_migrations::<TestMigrator>()
            .await
            .unwrap();
    }
}
