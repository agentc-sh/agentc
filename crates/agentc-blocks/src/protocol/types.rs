// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::composition::GenerationContribution;

pub struct ResolvedProtocol {
    pub name: String,
    pub contribution: GenerationContribution,
}
