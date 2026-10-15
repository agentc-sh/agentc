// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod block;

use agentc_compiler::generator::blocks::Block;

use crate::{context::ResolvedContext, custom::block::CustomBlock};

pub struct CustomBlocks;

impl CustomBlocks {
    pub fn resolve(context: &ResolvedContext) -> Vec<Box<dyn Block<ResolvedContext>>> {
        context
            .blocks
            .values()
            .map(|block| Box::new(CustomBlock::from(block)) as Box<dyn Block<ResolvedContext>>)
            .collect()
    }
}
