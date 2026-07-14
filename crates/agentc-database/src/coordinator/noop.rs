// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use sea_orm_migration::MigratorTrait;

use crate::{
    coordinator::MigrationCoordinator,
    errors::DatabaseError,
    orm::DatabaseConnection,
};

pub struct NoopCoordinator;

#[async_trait]
impl MigrationCoordinator for NoopCoordinator {
    async fn run<M: MigratorTrait>(
        &self,
        conn: &DatabaseConnection,
    ) -> Result<(), DatabaseError> {
        M::up(conn, None)
            .await
            .map_err(DatabaseError::from)
    }
}
