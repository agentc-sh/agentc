// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use agentc_compiler::{generator::blocks::Block, toolchain::traits::ErasedToolchain};

use crate::{
    archetype::types::ResolvedArchetype, context::ResolvedContext, errors::BlocksError,
    feature::GenerationFeatureSet, graph::types::ResolvedGraph, protocol::types::ResolvedProtocol,
    runtime::EmbeddedAsset,
};

pub struct GenerationContribution {
    pub blocks: Vec<Box<dyn Block<ResolvedContext>>>,
    pub embedded_assets: Vec<&'static EmbeddedAsset>,
    pub provides: GenerationFeatureSet,
    pub requires: GenerationFeatureSet,
}

impl GenerationContribution {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            embedded_assets: Vec::new(),
            provides: GenerationFeatureSet::new(),
            requires: GenerationFeatureSet::new(),
        }
    }

    pub fn with_blocks(mut self, blocks: Vec<Box<dyn Block<ResolvedContext>>>) -> Self {
        self.blocks = blocks;
        self
    }

    pub fn with_embedded_assets(mut self, embedded_assets: Vec<&'static EmbeddedAsset>) -> Self {
        self.embedded_assets = embedded_assets;
        self
    }

    pub fn with_provides(mut self, provides: GenerationFeatureSet) -> Self {
        self.provides = provides;
        self
    }

    pub fn with_requires(mut self, requires: GenerationFeatureSet) -> Self {
        self.requires = requires;
        self
    }
}

impl Default for GenerationContribution {
    fn default() -> Self {
        Self::new()
    }
}

pub struct OptionalGenerationContribution {
    pub contribution: GenerationContribution,
}

impl OptionalGenerationContribution {
    pub fn new(contribution: GenerationContribution) -> Self {
        Self { contribution }
    }
}

pub struct CompositionInput {
    pub archetype: ResolvedArchetype,
    pub graph: ResolvedGraph,
    pub protocols: Vec<ResolvedProtocol>,
    pub blocks: Vec<Box<dyn Block<ResolvedContext>>>,
}

pub struct ComposedGeneration {
    pub archetype_name: String,
    pub graph_name: String,
    pub protocol_names: Vec<String>,
    pub toolchain: Box<dyn ErasedToolchain>,
    pub blocks: Vec<Box<dyn Block<ResolvedContext>>>,
    pub embedded_assets: Vec<&'static EmbeddedAsset>,
}

pub struct Composer;

impl Composer {
    pub fn new() -> Self {
        Self
    }

    pub fn compose(self, mut input: CompositionInput) -> Result<ComposedGeneration, BlocksError> {
        let archetype_name = input.archetype.name.clone();
        let graph_name = input.graph.name.clone();
        let protocol_names = input
            .protocols
            .iter()
            .map(|protocol| protocol.name.clone())
            .collect::<Vec<_>>();
        let toolchain = input.archetype.toolchain;
        let mut archetype_contribution = input.archetype.contribution;
        let mut graph_contribution = input.graph.contribution;

        let mut blocks = Vec::new();
        let mut embedded_assets = Vec::new();
        let mut embedded_asset_names = HashMap::new();
        let mut provided = GenerationFeatureSet::new();

        Self::apply_required_contribution(
            "archetype",
            &archetype_name,
            &mut archetype_contribution,
            &mut blocks,
            &mut embedded_assets,
            &mut embedded_asset_names,
            &mut provided,
        )?;

        Self::apply_required_contribution(
            "graph",
            &graph_name,
            &mut graph_contribution,
            &mut blocks,
            &mut embedded_assets,
            &mut embedded_asset_names,
            &mut provided,
        )?;

        for integration in &mut input.graph.integrations {
            if integration
                .contribution
                .requires
                .missing_requirements(&provided)
                .is_empty()
            {
                Self::apply_contribution(
                    &mut integration.contribution,
                    &mut blocks,
                    &mut embedded_assets,
                    &mut embedded_asset_names,
                    &mut provided,
                )?;
            }
        }

        for protocol in &mut input.protocols {
            Self::apply_required_contribution(
                "protocol",
                &protocol.name,
                &mut protocol.contribution,
                &mut blocks,
                &mut embedded_assets,
                &mut embedded_asset_names,
                &mut provided,
            )?;
        }

        blocks.extend(input.blocks);

        Ok(ComposedGeneration {
            archetype_name,
            graph_name,
            protocol_names,
            toolchain,
            blocks,
            embedded_assets,
        })
    }

    fn apply_required_contribution(
        component: &str,
        name: &str,
        contribution: &mut GenerationContribution,
        blocks: &mut Vec<Box<dyn Block<ResolvedContext>>>,
        embedded_assets: &mut Vec<&'static EmbeddedAsset>,
        embedded_asset_names: &mut HashMap<&'static str, &'static EmbeddedAsset>,
        provided: &mut GenerationFeatureSet,
    ) -> Result<(), BlocksError> {
        let missing = contribution
            .requires
            .missing_requirements(provided);

        if !missing.is_empty() {
            return Err(BlocksError::invalid(format!(
                "{component} {name:?} requires missing generation features: {}",
                missing.join(", "),
            )));
        }

        Self::apply_contribution(
            contribution,
            blocks,
            embedded_assets,
            embedded_asset_names,
            provided,
        )
    }

    fn apply_contribution(
        contribution: &mut GenerationContribution,
        blocks: &mut Vec<Box<dyn Block<ResolvedContext>>>,
        embedded_assets: &mut Vec<&'static EmbeddedAsset>,
        embedded_asset_names: &mut HashMap<&'static str, &'static EmbeddedAsset>,
        provided: &mut GenerationFeatureSet,
    ) -> Result<(), BlocksError> {
        blocks.append(&mut contribution.blocks);

        for asset in contribution.embedded_assets.drain(..) {
            if let Some(existing) = embedded_asset_names.get(asset.name) {
                if !std::ptr::eq(*existing, asset) {
                    return Err(BlocksError::invalid(format!(
                        "embedded asset name conflict: {:?}",
                        asset.name,
                    )));
                }

                continue;
            }

            embedded_asset_names.insert(asset.name, asset);
            embedded_assets.push(asset);
        }

        *provided = provided.union(&contribution.provides);

        Ok(())
    }
}

impl Default for Composer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::*;
    use crate::{
        feature::{Cli, GraphReAct, HttpServer, Streaming},
        runtime::ExtractionMode,
    };
    use agentc_compiler::{
        compiler::{
            errors::CompilerError,
            traits::{Compiler, OutputSink},
            types::CompileParams,
        },
        generator::{
            blocks::traits::Block, context::GenerationContext, errors::GeneratorError,
            extension::ExtensionRegistry, vfs::VirtualFileSystem,
        },
        runner::{
            errors::RunnerError,
            traits::Runner,
            types::{RunOutcome, RunParams},
        },
        toolchain::traits::{ErasedToolchainCell, Toolchain},
    };

    struct StubCompiler;

    #[async_trait]
    impl Compiler for StubCompiler {
        type Artifact = ();

        async fn compile(
            &self,
            _params: CompileParams,
            _output_sink: &dyn OutputSink,
        ) -> Result<Self::Artifact, CompilerError> {
            Err(CompilerError::compilation_failed("not used in composition tests"))
        }
    }

    struct StubRunner;

    #[async_trait]
    impl Runner for StubRunner {
        type Artifact = ();

        async fn run(
            &self,
            _artifact: &Self::Artifact,
            _params: RunParams,
        ) -> Result<RunOutcome, RunnerError> {
            Err(RunnerError::invocation_failed("not used in composition tests"))
        }
    }

    struct StubToolchain {
        compiler: StubCompiler,
        runner: Option<StubRunner>,
    }

    impl StubToolchain {
        fn new(runnable: bool) -> Self {
            Self {
                compiler: StubCompiler,
                runner: runnable.then_some(StubRunner),
            }
        }
    }

    impl Toolchain for StubToolchain {
        type Artifact = ();

        fn compiler(&self) -> &dyn Compiler<Artifact = Self::Artifact> {
            &self.compiler
        }

        fn runner(&self) -> Option<&dyn Runner<Artifact = Self::Artifact>> {
            self.runner
                .as_ref()
                .map(|runner| runner as &dyn Runner<Artifact = Self::Artifact>)
        }
    }

    struct StubBlock {
        id: &'static str,
    }

    #[async_trait]
    impl Block<ResolvedContext> for StubBlock {
        fn id(&self) -> &str {
            self.id
        }

        async fn render(
            &self,
            _ctx: &GenerationContext<ResolvedContext>,
            _registry: &ExtensionRegistry,
            _vfs: &mut VirtualFileSystem,
        ) -> Result<(), GeneratorError> {
            Ok(())
        }
    }

    static SHARED_ASSET: EmbeddedAsset = EmbeddedAsset {
        name: "shared",
        bytes: b"shared",
        mode: ExtractionMode::Raw,
    };

    static CONFLICTING_SHARED_ASSET: EmbeddedAsset = EmbeddedAsset {
        name: "shared",
        bytes: b"other",
        mode: ExtractionMode::Raw,
    };

    fn block(id: &'static str) -> Box<dyn Block<ResolvedContext>> {
        Box::new(StubBlock { id })
    }

    fn archetype(contribution: GenerationContribution, runnable: bool) -> ResolvedArchetype {
        ResolvedArchetype {
            name: "standalone".to_string(),
            toolchain: ErasedToolchainCell::erase(StubToolchain::new(runnable)),
            contribution,
        }
    }

    fn graph(
        contribution: GenerationContribution,
        integrations: Vec<OptionalGenerationContribution>,
    ) -> ResolvedGraph {
        ResolvedGraph {
            name: "react".to_string(),
            contribution,
            integrations,
        }
    }

    fn protocol(contribution: GenerationContribution) -> ResolvedProtocol {
        ResolvedProtocol { name: "ag_ui".to_string(), contribution }
    }

    fn provides<T>() -> GenerationFeatureSet
    where
        T: crate::feature::GenerationFeature,
    {
        let mut features = GenerationFeatureSet::new();

        features.insert::<T>();

        features
    }

    #[test]
    fn composer_preserves_deterministic_order() {
        let composed = Composer::new()
            .compose(CompositionInput {
                archetype: archetype(
                    GenerationContribution::new().with_blocks(vec![block("archetype")]),
                    true,
                ),
                graph: graph(
                    GenerationContribution::new().with_blocks(vec![block("graph")]),
                    vec![OptionalGenerationContribution::new(
                        GenerationContribution::new().with_blocks(vec![block("integration")]),
                    )],
                ),
                protocols: vec![protocol(
                    GenerationContribution::new().with_blocks(vec![block("protocol")]),
                )],
                blocks: vec![block("custom")],
            })
            .unwrap();

        let ids = composed
            .blocks
            .iter()
            .map(|block| block.id().to_string())
            .collect::<Vec<_>>();

        assert_eq!(ids, vec!["archetype", "graph", "integration", "protocol", "custom"],);
    }

    #[test]
    fn composer_rejects_missing_required_features() {
        let result = Composer::new().compose(CompositionInput {
            archetype: archetype(GenerationContribution::new(), true),
            graph: graph(
                GenerationContribution::new().with_requires(provides::<Cli>()),
                Vec::new(),
            ),
            protocols: Vec::new(),
            blocks: Vec::new(),
        });

        assert!(matches!(result, Err(BlocksError::InvalidManifest { .. })));
    }

    #[test]
    fn composer_skips_optional_integrations_with_missing_features() {
        let composed = Composer::new()
            .compose(CompositionInput {
                archetype: archetype(GenerationContribution::new(), true),
                graph: graph(
                    GenerationContribution::new().with_blocks(vec![block("graph")]),
                    vec![OptionalGenerationContribution::new(
                        GenerationContribution::new()
                            .with_blocks(vec![block("integration")])
                            .with_requires(provides::<HttpServer>()),
                    )],
                ),
                protocols: Vec::new(),
                blocks: Vec::new(),
            })
            .unwrap();

        let ids = composed
            .blocks
            .iter()
            .map(|block| block.id().to_string())
            .collect::<Vec<_>>();

        assert_eq!(ids, vec!["graph"]);
    }

    #[test]
    fn composer_rejects_incompatible_protocols() {
        let result = Composer::new().compose(CompositionInput {
            archetype: archetype(
                GenerationContribution::new().with_provides(provides::<HttpServer>()),
                true,
            ),
            graph: graph(
                GenerationContribution::new().with_provides(provides::<Streaming>()),
                Vec::new(),
            ),
            protocols: vec![protocol(GenerationContribution::new().with_requires({
                let mut features = provides::<HttpServer>();

                features.insert::<GraphReAct>();
                features
            }))],
            blocks: Vec::new(),
        });

        assert!(matches!(result, Err(BlocksError::InvalidManifest { .. })));
    }

    #[test]
    fn composer_deduplicates_repeated_asset_references() {
        let composed = Composer::new()
            .compose(CompositionInput {
                archetype: archetype(
                    GenerationContribution::new().with_embedded_assets(vec![&SHARED_ASSET]),
                    true,
                ),
                graph: graph(
                    GenerationContribution::new().with_embedded_assets(vec![&SHARED_ASSET]),
                    Vec::new(),
                ),
                protocols: Vec::new(),
                blocks: Vec::new(),
            })
            .unwrap();

        assert_eq!(composed.embedded_assets.len(), 1);
        assert!(std::ptr::eq(composed.embedded_assets[0], &SHARED_ASSET));
    }

    #[test]
    fn composer_rejects_conflicting_asset_names() {
        let result = Composer::new().compose(CompositionInput {
            archetype: archetype(
                GenerationContribution::new().with_embedded_assets(vec![&SHARED_ASSET]),
                true,
            ),
            graph: graph(
                GenerationContribution::new().with_embedded_assets(vec![&CONFLICTING_SHARED_ASSET]),
                Vec::new(),
            ),
            protocols: Vec::new(),
            blocks: Vec::new(),
        });

        assert!(matches!(result, Err(BlocksError::InvalidManifest { .. })));
    }

    #[test]
    fn composer_preserves_archetype_toolchain() {
        let composed = Composer::new()
            .compose(CompositionInput {
                archetype: archetype(GenerationContribution::new(), true),
                graph: graph(GenerationContribution::new(), Vec::new()),
                protocols: Vec::new(),
                blocks: Vec::new(),
            })
            .unwrap();

        assert_eq!(composed.archetype_name, "standalone");
        assert_eq!(composed.graph_name, "react");
        assert_eq!(composed.protocol_names, Vec::<String>::new());
        assert!(composed.toolchain.supports_run());

        assert!(
            !Composer::new()
                .compose(CompositionInput {
                    archetype: archetype(GenerationContribution::new(), false),
                    graph: graph(GenerationContribution::new(), Vec::new()),
                    protocols: Vec::new(),
                    blocks: Vec::new(),
                })
                .unwrap()
                .toolchain
                .supports_run()
        );
    }
}
