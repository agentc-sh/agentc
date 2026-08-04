// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_compiler::toolchain::traits::ErasedToolchain;

use crate::composition::GenerationContribution;

pub struct ResolvedArchetype {
    pub name: String,
    pub toolchain: Box<dyn ErasedToolchain>,
    pub contribution: GenerationContribution,
}
