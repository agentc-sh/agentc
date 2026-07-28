// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_database::migrations::MigrationTrait;

mod m20260323_000001_initial;
mod m20260713_000002_model_config;
mod m20260726_000003_message_checkpoint_id;

pub fn all() -> Vec<Box<dyn MigrationTrait>> {
    vec![
        Box::new(m20260323_000001_initial::Migration),
        Box::new(m20260713_000002_model_config::Migration),
        Box::new(m20260726_000003_message_checkpoint_id::Migration),
    ]
}
