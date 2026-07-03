// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[allow(unused_extern_crates)]
extern crate self as agentc_database;

pub mod connection;
pub mod database;
pub mod errors;
pub mod json;
pub mod orm;
pub mod paginate;
pub mod query;

pub mod migrations {
    pub use sea_orm_migration::prelude::*;
}

pub use database::Database;
