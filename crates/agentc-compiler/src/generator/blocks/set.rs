// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::Serialize;

use crate::generator::blocks::traits::Block;

/// A fluent collection of blocks for a single generation run.
///
/// Provides `add` and `add_if` methods to assemble a block list inline without
/// requiring a named factory method for each entry. Call `into_inner` to produce
/// the `Vec<Box<dyn Block<T>>>` consumed by `BlockGraph::try_from`.
pub struct BlockSet<T>(Vec<Box<dyn Block<T>>>)
where
    T: Serialize + Send + Sync + 'static;

impl<T: Serialize + Send + Sync + 'static> BlockSet<T> {
    pub fn new() -> Self {
        Self(vec![])
    }

    /// Append a block unconditionally.
    pub fn add(mut self, block: impl Block<T> + 'static) -> Self {
        self.0.push(Box::new(block));
        self
    }

    /// Append a block only when `condition` is true.
    pub fn add_if(self, condition: bool, block: impl Block<T> + 'static) -> Self {
        if condition {
            self.add(block)
        } else {
            self
        }
    }

    /// Consume the set and return the underlying vec.
    pub fn into_inner(self) -> Vec<Box<dyn Block<T>>> {
        self.0
    }
}

impl<T: Serialize + Send + Sync + 'static> Default for BlockSet<T> {
    fn default() -> Self {
        Self::new()
    }
}
