// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_compiler::{compiler::traits::Compiler, generator::blocks::Block};

use crate::{context::ResolvedContext, runtime::EmbeddedAsset};

/// The output of a successful archetype resolution, containing
/// the resolved archetype name and the list of instantiated blocks.
pub struct ResolvedArchetype {
    /// Identifier of the archetype that was selected.
    pub name: String,
    /// The compiler that should be used to compile the blocks returned by this archetype.
    pub compiler: Box<dyn Compiler>,
    /// Ordered list of blocks to generate with.
    pub blocks: Vec<Box<dyn Block<ResolvedContext>>>,
    /// The resolved target we are compiling for. This is optional because some archetypes may not
    /// specify a target, in which case the target is determined by which archetype/compiler is used.
    pub target: Option<String>,
    /// Binary assets embedded in the compiler that must be materialized on disk before compilation.
    /// Archetypes with no embedded assets set this to `&[]`.
    pub embedded_assets: &'static [EmbeddedAsset],
}

impl ResolvedArchetype {
    pub fn extend_blocks(&mut self, blocks: Vec<Box<dyn Block<ResolvedContext>>>) {
        self.blocks.extend(blocks);
    }

    pub fn with_blocks(mut self, blocks: Vec<Box<dyn Block<ResolvedContext>>>) -> Self {
        self.extend_blocks(blocks);
        self
    }
}
