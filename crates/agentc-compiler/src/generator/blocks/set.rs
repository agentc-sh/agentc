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
        if condition { self.add(block) } else { self }
    }

    /// Append a block built from `value` only when it is `Some`.
    pub fn add_some<V, B>(self, value: Option<V>, block: impl FnOnce(V) -> B) -> Self
    where
        B: Block<T> + 'static,
    {
        match value {
            Some(value) => self.add(block(value)),
            None => self,
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

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::*;
    use crate::generator::{
        context::GenerationContext, errors::GeneratorError, extension::ExtensionRegistry,
        vfs::VirtualFileSystem,
    };

    struct StubBlock {
        id: &'static str,
    }

    #[async_trait]
    impl Block<()> for StubBlock {
        fn id(&self) -> &str {
            self.id
        }

        async fn render(
            &self,
            _ctx: &GenerationContext<()>,
            _registry: &ExtensionRegistry,
            _vfs: &mut VirtualFileSystem,
        ) -> Result<(), GeneratorError> {
            Ok(())
        }
    }

    fn ids(set: BlockSet<()>) -> Vec<String> {
        set.into_inner()
            .iter()
            .map(|block| block.id().to_string())
            .collect()
    }

    #[test]
    fn add_if_appends_only_when_true() {
        let set = BlockSet::new()
            .add_if(true, StubBlock { id: "a" })
            .add_if(false, StubBlock { id: "b" });

        assert_eq!(ids(set), vec!["a"]);
    }

    #[test]
    fn add_some_appends_only_when_some() {
        let set = BlockSet::new()
            .add_some(Some("config"), |value| StubBlock { id: value })
            .add_some(None::<&'static str>, |value| StubBlock { id: value });

        assert_eq!(ids(set), vec!["config"]);
    }
}
