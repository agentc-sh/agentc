// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod codegen;
pub mod graph;
pub mod set;
pub mod template;
pub mod traits;

pub use graph::BlockGraph;
pub use set::BlockSet;
pub use traits::Block;

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde::Serialize;

    use crate::generator::{
        context::GenerationContext,
        errors::GeneratorError,
        extension::{
            Contribution,
            ErasedContribution,
            ErasedExtensionPoint,
            ExtensionRegistry,
            ExtensionPoint,
            StringExtensionPoint,
            reducers,
        },
        vfs::VirtualFileSystem,
    };

    #[derive(Clone)]
    struct NumberExtensionPoint;

    impl ExtensionPoint for NumberExtensionPoint {
        type Contribution = u64;

        fn name(&self) -> &str {
            "number"
        }

        fn reduce(
            &self,
            contributions: Vec<Self::Contribution>,
        ) -> Result<String, GeneratorError> {
            Ok(
                contributions
                    .into_iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            )
        }
    }

    /// Minimal block that only declares an ID.
    struct StubBlock {
        id: String,
        extension_points: Vec<Box<dyn ErasedExtensionPoint>>,
        contributions: Vec<ErasedContribution>,
    }

    impl StubBlock {
        fn new(id: &str) -> Self {
            Self {
                id: id.into(),
                extension_points: vec![],
                contributions: vec![],
            }
        }

        fn with_extension_point(mut self, name: &str) -> Self {
            self.extension_points
                .push(Box::new(StringExtensionPoint::new(
                    name,
                    reducers::concat,
                )));
            self
        }

        fn with_number_extension_point(mut self) -> Self {
            self.extension_points
                .push(Box::new(NumberExtensionPoint));
            self
        }

        fn with_strict_contribution(mut self, point: &str) -> Self {
            self.contributions
                .push(Contribution::<String>::strict(point).erase());
            self
        }

        fn with_lenient_contribution(mut self, point: &str) -> Self {
            self.contributions
                .push(Contribution::<String>::lenient(point).erase());
            self
        }

        fn with_number_contribution(mut self, point: &str) -> Self {
            self.contributions
                .push(Contribution::<u64>::strict(point).erase());
            self
        }
    }

    #[async_trait]
    impl<T: Serialize + Send + Sync> Block<T> for StubBlock {
        fn id(&self) -> &str {
            &self.id
        }

        fn extension_points(&self) -> Vec<Box<dyn ErasedExtensionPoint>> {
            self.extension_points.clone()
        }

        fn contributions(&self) -> Vec<ErasedContribution> {
            self.contributions.clone()
        }

        async fn render(
            &self,
            _ctx: &GenerationContext<T>,
            _registry: &ExtensionRegistry,
            _vfs: &mut VirtualFileSystem,
        ) -> Result<(), GeneratorError> {
            Ok(())
        }
    }

    fn boxed<T: Serialize + Send + Sync + 'static>(b: StubBlock) -> Box<dyn Block<T>> {
        Box::new(b)
    }

    #[test]
    fn valid_graph_constructs_successfully() {
        type T = ();
        let blocks: Vec<Box<dyn Block<T>>> =
            vec![boxed(StubBlock::new("a")), boxed(StubBlock::new("b"))];
        assert!(BlockGraph::try_from(blocks).is_ok());
    }

    #[test]
    fn duplicate_block_id_fails() {
        type T = ();
        let blocks: Vec<Box<dyn Block<T>>> =
            vec![boxed(StubBlock::new("a")), boxed(StubBlock::new("a"))];
        let result = BlockGraph::try_from(blocks);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, GeneratorError::DuplicateBlock(id) if id == "a"));
    }

    #[test]
    fn strict_contribution_to_declared_point_succeeds() {
        type T = ();
        let blocks: Vec<Box<dyn Block<T>>> = vec![
            boxed(StubBlock::new("declarer").with_extension_point("my_point")),
            boxed(StubBlock::new("contributor").with_strict_contribution("my_point")),
        ];
        assert!(BlockGraph::try_from(blocks).is_ok());
    }

    #[test]
    fn strict_contribution_to_undeclared_point_fails() {
        type T = ();
        let blocks: Vec<Box<dyn Block<T>>> = vec![boxed(
            StubBlock::new("contributor").with_strict_contribution("ghost_point"),
        )];
        let result = BlockGraph::try_from(blocks);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(
            err,
            GeneratorError::UndeclaredExtensionPoint { block_id, point }
                if block_id == "contributor" && point == "ghost_point"
        ));
    }

    #[test]
    fn lenient_contribution_to_undeclared_point_succeeds() {
        type T = ();
        let blocks: Vec<Box<dyn Block<T>>> = vec![boxed(
            StubBlock::new("contributor").with_lenient_contribution("ghost_point"),
        )];
        assert!(BlockGraph::try_from(blocks).is_ok());
    }

    #[test]
    fn duplicate_extension_point_fails() {
        type T = ();
        let blocks: Vec<Box<dyn Block<T>>> = vec![
            boxed(StubBlock::new("a").with_extension_point("my_point")),
            boxed(StubBlock::new("b").with_extension_point("my_point")),
        ];
        let result = BlockGraph::try_from(blocks);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(
            err,
            GeneratorError::DuplicateExtensionPoint { point } if point == "my_point"
        ));
    }

    #[test]
    fn contribution_type_mismatch_fails() {
        type T = ();
        let blocks: Vec<Box<dyn Block<T>>> = vec![
            boxed(StubBlock::new("declarer").with_number_extension_point()),
            boxed(StubBlock::new("contributor").with_strict_contribution("number")),
        ];
        let result = BlockGraph::try_from(blocks);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(
            err,
            GeneratorError::ExtensionPointTypeMismatch { block_id, point, .. }
                if block_id == "contributor" && point == "number"
        ));
    }

    #[test]
    fn typed_contribution_to_matching_point_succeeds() {
        type T = ();
        let blocks: Vec<Box<dyn Block<T>>> = vec![
            boxed(StubBlock::new("declarer").with_number_extension_point()),
            boxed(StubBlock::new("contributor").with_number_contribution("number")),
        ];
        assert!(BlockGraph::try_from(blocks).is_ok());
    }

    #[test]
    fn empty_graph_is_valid() {
        type T = ();
        let blocks: Vec<Box<dyn Block<T>>> = vec![];
        assert!(BlockGraph::try_from(blocks).is_ok());
    }
}
