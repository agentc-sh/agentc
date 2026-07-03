// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_database::migrations::MigrationTrait;

mod m20260323_000001_initial;

pub fn all() -> Vec<Box<dyn MigrationTrait>> {
    vec![Box::new(m20260323_000001_initial::Migration)]
}
