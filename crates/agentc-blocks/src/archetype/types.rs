// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_compiler::compiler::traits::Compiler;

use crate::composition::GenerationContribution;

pub struct ResolvedArchetype {
    pub name: String,
    pub compiler: Box<dyn Compiler>,
    pub target: Option<String>,
    pub contribution: GenerationContribution,
}
