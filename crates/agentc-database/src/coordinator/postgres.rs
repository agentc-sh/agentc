// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use sea_orm_migration::MigratorTrait;
use sqlx::postgres::PgAdvisoryLock;

use crate::{coordinator::MigrationCoordinator, errors::DatabaseError, orm::DatabaseConnection};

const MIGRATION_LOCK_KEY: &str = "agentc::database::migrations";

/// Serializes migrations across replicas with a session-level Postgres
/// advisory lock.
///
/// The primary pool must permit at least two connections during migration: one
/// is held by the lock guard while the other runs `M::up`. A single-connection
/// pool would deadlock.
pub struct PostgresAdvisoryLockCoordinator;

#[async_trait]
impl MigrationCoordinator for PostgresAdvisoryLockCoordinator {
    async fn run<M: MigratorTrait>(&self, conn: &DatabaseConnection) -> Result<(), DatabaseError> {
        let lock = PgAdvisoryLock::new(MIGRATION_LOCK_KEY);

        let guard = lock
            .acquire(
                conn.get_postgres_connection_pool()
                    .acquire()
                    .await
                    .map_err(|e| DatabaseError::unexpected_error(e.to_string()))?,
            )
            .await
            .map_err(|e| DatabaseError::unexpected_error(e.to_string()))?;

        let result = M::up(conn, None)
            .await
            .map_err(DatabaseError::from);

        guard
            .release_now()
            .await
            .map_err(|e| DatabaseError::unexpected_error(e.to_string()))?;

        result
    }
}
