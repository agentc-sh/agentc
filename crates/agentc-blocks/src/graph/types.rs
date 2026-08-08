// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::composition::GenerationContribution;

pub struct ResolvedGraph {
    pub name: String,
    pub contribution: GenerationContribution,
    pub integrations: Vec<GenerationContribution>,
}
