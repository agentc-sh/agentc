// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::Serialize;
use std::collections::HashSet;

use crate::generator::{blocks::traits::Block, errors::GeneratorError};

/// A validated set of blocks ready for composition.
pub struct BlockGraph<T>
where
    T: Serialize + Send + Sync,
{
    blocks: Vec<Box<dyn Block<T>>>,
}

impl<T> BlockGraph<T>
where
    T: Serialize + Send + Sync,
{
    pub fn try_new(blocks: Vec<Box<dyn Block<T>>>) -> Result<Self, GeneratorError> {
        blocks.try_into()
    }

    /// Validate the block graph.
    pub fn validate(&self) -> Result<(), GeneratorError> {
        let mut seen_blocks = HashSet::new();
        let declared_extension_points = self
            .blocks
            .iter()
            .flat_map(|block| block.extension_points())
            .map(|point| point.name().to_string())
            .collect::<HashSet<_>>();

        for block in &self.blocks {
            if !seen_blocks.insert(block.id()) {
                return Err(GeneratorError::DuplicateBlock(block.id().to_string()));
            }

            for contribution in block.contributions() {
                if contribution.strict && !declared_extension_points.contains(&contribution.point) {
                    return Err(GeneratorError::UndeclaredExtensionPoint {
                        block_id: block.id().to_string(),
                        point: contribution.point.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Get the blocks in this graph.
    pub fn blocks(&self) -> &[Box<dyn Block<T>>] {
        &self.blocks
    }
}

impl<T> TryFrom<Vec<Box<dyn Block<T>>>> for BlockGraph<T>
where
    T: Serialize + Send + Sync,
{
    type Error = GeneratorError;

    fn try_from(value: Vec<Box<dyn Block<T>>>) -> Result<Self, Self::Error> {
        let graph = Self { blocks: value };
        graph.validate()?;
        Ok(graph)
    }
}
