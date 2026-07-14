// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use sea_orm_migration::MigratorTrait;

use crate::{errors::DatabaseError, orm::DatabaseConnection};

pub mod noop;
pub mod postgres;

pub use noop::NoopCoordinator;
pub use postgres::PostgresAdvisoryLockCoordinator;

/// Brackets a migration run with whatever cross-process serialization the
/// backend requires.
///
/// The generic `run<M>` method makes this trait intentionally non-object-safe;
/// it is selected and dispatched statically, never through a `dyn` object.
#[async_trait]
pub trait MigrationCoordinator {
    async fn run<M: MigratorTrait>(
        &self,
        conn: &DatabaseConnection,
    ) -> Result<(), DatabaseError>;
}
