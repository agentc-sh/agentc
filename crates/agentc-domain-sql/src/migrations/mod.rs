// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_database::migrations::MigrationTrait;

mod m20260322_000001_initial;

pub fn all() -> Vec<Box<dyn MigrationTrait>> {
    vec![Box::new(m20260322_000001_initial::Migration)]
}
