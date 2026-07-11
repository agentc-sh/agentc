// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::Serialize;
use std::collections::HashMap;

use crate::generator::{
    blocks::{Block, BlockGraph},
    context::GenerationContext,
    errors::GeneratorError,
    extension::{
        ErasedContributionValue,
        ExtensionRegistry,
    },
    vfs::VirtualFileSystem,
};

/// The core generation engine.
pub struct Generator<T>
where
    T: Serialize + Send + Sync,
{
    ctx: GenerationContext<T>,
    blocks: Vec<Box<dyn Block<T>>>,
}

impl<T> Generator<T>
where
    T: Serialize + Send + Sync,
{
    /// Create a new generator builder.
    pub fn builder() -> GeneratorBuilder<T> {
        GeneratorBuilder::new()
    }

    /// Generate the output files based on the provided blocks and context.
    pub async fn generate(self) -> Result<VirtualFileSystem, GeneratorError> {
        let mut contributions = HashMap::<String, Vec<ErasedContributionValue>>::new();
        let graph = BlockGraph::try_from(self.blocks)?;
        let points = graph
            .blocks()
            .iter()
            .flat_map(|block| block.extension_points())
            .collect::<Vec<_>>();

        for block in graph.blocks() {
            for contribution in block.contributions() {
                if points
                    .iter()
                    .any(|point| point.name() == contribution.point)
                {
                    contributions
                        .entry(contribution.point.clone())
                        .or_default()
                        .push(
                            block
                                .render_contribution(&self.ctx, &contribution.point)
                                .await?,
                        );
                }
            }
        }

        let registry = ExtensionRegistry::resolve(points, contributions)?;
        let mut vfs = VirtualFileSystem::new();

        for block in graph.blocks() {
            block
                .render(&self.ctx, &registry, &mut vfs)
                .await?;
        }

        Ok(vfs)
    }
}

/// A builder for constructing a [`Generator`](crate::generator::generator::Generator)
/// with a fluent API.
pub struct GeneratorBuilder<T>
where
    T: Serialize + Send + Sync,
{
    ctx: Option<GenerationContext<T>>,
    blocks: Vec<Box<dyn Block<T>>>,
}

impl<T> GeneratorBuilder<T>
where
    T: Serialize + Send + Sync,
{
    /// Create a new generator builder.
    pub fn new() -> Self {
        Self { ctx: None, blocks: Vec::new() }
    }

    /// Set the generation context for this generator.
    pub fn with_context(mut self, ctx: T) -> Self {
        self.ctx = Some(GenerationContext::new(ctx));
        self
    }

    /// Add a block to the generator.
    pub fn with_block<B>(mut self, block: B) -> Self
    where
        B: Block<T> + 'static,
    {
        self.blocks.push(Box::new(block));
        self
    }

    /// Add multiple blocks to the generator.
    pub fn with_blocks<I, B>(mut self, blocks: I) -> Self
    where
        I: IntoIterator<Item = B>,
        B: Into<Box<dyn Block<T>>>,
    {
        self.blocks
            .extend(blocks.into_iter().map(Into::into));
        self
    }

    /// Build the generator. This will panic if the generation context is not set.
    pub fn build(self) -> Generator<T> {
        Generator {
            ctx: self
                .ctx
                .expect("Generation context is required"),
            blocks: self.blocks,
        }
    }
}

impl<T> Default for GeneratorBuilder<T>
where
    T: Serialize + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use serde::Serialize;
    use std::sync::{Arc, Mutex};

    use crate::generator::{
        blocks::Block,
        context::GenerationContext,
        errors::GeneratorError,
        extension::{
            Contribution,
            ErasedContribution,
            ErasedContributionValue,
            ErasedExtensionPoint,
            ExtensionRegistry,
            StringExtensionPoint,
            reducers,
        },
        pipeline::Generator,
        vfs::VirtualFileSystem,
    };

    #[derive(Clone, Serialize)]
    struct Cfg {
        name: String,
    }

    /// Tracks render call order for pipeline ordering assertions.
    #[derive(Clone)]
    struct OrderTracker(Arc<Mutex<Vec<String>>>);

    impl OrderTracker {
        fn new() -> Self {
            Self(Arc::new(Mutex::new(vec![])))
        }
        fn record(&self, s: &str) {
            self.0.lock().unwrap().push(s.into());
        }
        fn recorded(&self) -> Vec<String> {
            self.0.lock().unwrap().clone()
        }
    }

    struct DeclBlock {
        tracker: OrderTracker,
    }

    #[async_trait]
    impl Block<Cfg> for DeclBlock {
        fn id(&self) -> &str {
            "declarer"
        }

        fn extension_points(&self) -> Vec<Box<dyn ErasedExtensionPoint>> {
            vec![Box::new(StringExtensionPoint::new(
                "hook",
                reducers::concat,
            ))]
        }

        async fn render(
            &self,
            _ctx: &GenerationContext<Cfg>,
            registry: &ExtensionRegistry,
            vfs: &mut VirtualFileSystem,
        ) -> Result<(), GeneratorError> {
            self.tracker.record("declarer:render");
            let hook = registry.get("hook").unwrap_or("");
            vfs.insert("out.txt", format!("hook={}", hook));
            Ok(())
        }
    }

    struct ContribBlock {
        tracker: OrderTracker,
    }

    #[async_trait]
    impl Block<Cfg> for ContribBlock {
        fn id(&self) -> &str {
            "contributor"
        }

        fn contributions(&self) -> Vec<ErasedContribution> {
            vec![Contribution::<String>::strict("hook").erase()]
        }

        async fn render_contribution(
            &self,
            ctx: &GenerationContext<Cfg>,
            _point: &str,
        ) -> Result<ErasedContributionValue, GeneratorError> {
            self.tracker
                .record("contributor:render_contribution");
            Ok(ErasedContributionValue::new(format!(
                "hello-from-{}",
                ctx.name,
            )))
        }

        async fn render(
            &self,
            _ctx: &GenerationContext<Cfg>,
            _registry: &ExtensionRegistry,
            _vfs: &mut VirtualFileSystem,
        ) -> Result<(), GeneratorError> {
            self.tracker
                .record("contributor:render");
            Ok(())
        }
    }

    #[tokio::test]
    async fn contributions_are_resolved_before_render() {
        let tracker = OrderTracker::new();

        let vfs = Generator::builder()
            .with_context(Cfg { name: "world".into() })
            .with_block(DeclBlock { tracker: tracker.clone() })
            .with_block(ContribBlock { tracker: tracker.clone() })
            .build()
            .generate()
            .await
            .expect("generation failed");

        // Contribution render must happen before file render
        let order = tracker.recorded();
        let contrib_pos = order
            .iter()
            .position(|s| s == "contributor:render_contribution")
            .unwrap();
        let decl_render_pos = order
            .iter()
            .position(|s| s == "declarer:render")
            .unwrap();
        assert!(
            contrib_pos < decl_render_pos,
            "contributions must be collected before render phase"
        );

        // Contribution content must appear in the rendered file
        assert_eq!(vfs.get("out.txt"), Some("hook=hello-from-world"));
    }

    #[tokio::test]
    async fn lenient_contribution_to_missing_point_does_not_fail() {
        struct LenientBlock;

        #[async_trait]
        impl Block<Cfg> for LenientBlock {
            fn id(&self) -> &str {
                "lenient"
            }
            fn contributions(&self) -> Vec<ErasedContribution> {
                vec![Contribution::<String>::lenient("nonexistent").erase()]
            }
            async fn render(
                &self,
                _ctx: &GenerationContext<Cfg>,
                _registry: &ExtensionRegistry,
                vfs: &mut VirtualFileSystem,
            ) -> Result<(), GeneratorError> {
                vfs.insert("lenient.txt", "ok");
                Ok(())
            }
        }

        let vfs = Generator::builder()
            .with_context(Cfg { name: "x".into() })
            .with_block(LenientBlock)
            .build()
            .generate()
            .await
            .expect("lenient contribution to missing point should not fail");

        assert!(vfs.contains("lenient.txt"));
    }

    #[tokio::test]
    async fn strict_contribution_to_missing_point_fails() {
        struct StrictBlock;

        #[async_trait]
        impl Block<Cfg> for StrictBlock {
            fn id(&self) -> &str {
                "strict"
            }
            fn contributions(&self) -> Vec<ErasedContribution> {
                vec![Contribution::<String>::strict("nonexistent").erase()]
            }
            async fn render(
                &self,
                _ctx: &GenerationContext<Cfg>,
                _registry: &ExtensionRegistry,
                _vfs: &mut VirtualFileSystem,
            ) -> Result<(), GeneratorError> {
                Ok(())
            }
        }

        let result = Generator::builder()
            .with_context(Cfg { name: "x".into() })
            .with_block(StrictBlock)
            .build()
            .generate()
            .await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(
            matches!(err, GeneratorError::UndeclaredExtensionPoint { point, .. } if point == "nonexistent")
        );
    }

    #[tokio::test]
    async fn multiple_contributions_to_same_point_are_concatenated() {
        struct Declarer;
        struct ContribA;
        struct ContribB;

        #[async_trait]
        impl Block<Cfg> for Declarer {
            fn id(&self) -> &str {
                "declarer"
            }
            fn extension_points(&self) -> Vec<Box<dyn ErasedExtensionPoint>> {
                vec![Box::new(StringExtensionPoint::new(
                    "deps",
                    reducers::concat,
                ))]
            }
            async fn render(
                &self,
                _ctx: &GenerationContext<Cfg>,
                registry: &ExtensionRegistry,
                vfs: &mut VirtualFileSystem,
            ) -> Result<(), GeneratorError> {
                vfs.insert(
                    "deps.txt",
                    registry
                        .get("deps")
                        .unwrap_or("")
                        .to_string(),
                );
                Ok(())
            }
        }

        #[async_trait]
        impl Block<Cfg> for ContribA {
            fn id(&self) -> &str {
                "a"
            }
            fn contributions(&self) -> Vec<ErasedContribution> {
                vec![Contribution::<String>::strict("deps").erase()]
            }
            async fn render_contribution(
                &self,
                _ctx: &GenerationContext<Cfg>,
                _point: &str,
            ) -> Result<ErasedContributionValue, GeneratorError> {
                Ok(ErasedContributionValue::new("tokio".to_string()))
            }
            async fn render(
                &self,
                _ctx: &GenerationContext<Cfg>,
                _registry: &ExtensionRegistry,
                _vfs: &mut VirtualFileSystem,
            ) -> Result<(), GeneratorError> {
                Ok(())
            }
        }

        #[async_trait]
        impl Block<Cfg> for ContribB {
            fn id(&self) -> &str {
                "b"
            }
            fn contributions(&self) -> Vec<ErasedContribution> {
                vec![Contribution::<String>::strict("deps").erase()]
            }
            async fn render_contribution(
                &self,
                _ctx: &GenerationContext<Cfg>,
                _point: &str,
            ) -> Result<ErasedContributionValue, GeneratorError> {
                Ok(ErasedContributionValue::new("serde".to_string()))
            }
            async fn render(
                &self,
                _ctx: &GenerationContext<Cfg>,
                _registry: &ExtensionRegistry,
                _vfs: &mut VirtualFileSystem,
            ) -> Result<(), GeneratorError> {
                Ok(())
            }
        }

        let vfs = Generator::builder()
            .with_context(Cfg { name: "x".into() })
            .with_block(Declarer)
            .with_block(ContribA)
            .with_block(ContribB)
            .build()
            .generate()
            .await
            .expect("generation failed");

        assert_eq!(vfs.get("deps.txt"), Some("tokio\nserde"));
    }
}
