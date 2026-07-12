// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

use agentc_compiler::generator::vfs::VirtualFileSystem;

use crate::pipeline::steps::{
    cleanup::CleanupStepEvent, compose::ComposeStepEvent, extract::ExtractStepEvent,
    fetch::FetchStepEvent, generate::GenerateStepEvent, resolve::ResolveStepEvent,
    transform::TransformStepEvent,
};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum GenerateEvent {
    GenerateStarted {
        /// The name of the agent being generated, as defined in the manifest.
        agent_name: String,
    },
    /// Fetching remote sources defined in the manifest.
    FetchingSources,
    /// Fetched remote sources defined in the manifest.
    SourcesFetched {
        /// The number of sources fetched.
        count: usize,
    },
    /// Emitted when the assets are being transformed.
    TransformingAssets {
        /// The number of assets being transformed.
        count: usize,
    },
    /// Emitted when stdout is received from the transformer.
    TransformerStdout(String),
    /// Emitted when stderr is received from the transformer.
    TransformerStderr(String),
    /// Emitted when the assets have been transformed.
    AssetsTransformed {
        /// The number of assets transformed.
        count: usize,
    },
    /// Emitted when the manifest is being resolved.
    ResolvingManifest,
    /// Emitted when the manifest has been successfully resolved.
    ManifestResolved {
        /// Agent name defined in the manifest.
        agent_name: String,
        /// Archetype name defined in the manifest.
        archetype_name: String,
        /// The number of tools defined in the manifest.
        tool_count: usize,
        /// The number of custom blocks defined in the manifest.
        block_count: usize,
    },
    /// Emitted when the selected archetype, graph, and protocols are being composed.
    Composing,
    /// Emitted when the selected archetype, graph, and protocols have been composed.
    Composed {
        /// Archetype name used for this composition.
        archetype_name: String,
        /// Graph name used for this composition.
        graph_name: String,
        /// Protocol names used for this composition, in manifest order.
        protocol_names: Vec<String>,
        /// The total number of blocks that will be rendered, including user custom blocks.
        block_count: usize,
    },
    /// Emitted when embedded runtime crates are being extracted to disk.
    ExtractingRuntime {
        /// The number of embedded assets being extracted.
        asset_count: usize,
    },
    /// Emitted when embedded runtime crates have been extracted to disk.
    RuntimeExtracted {
        /// The directory where the runtime crates were extracted.
        runtime_dir: PathBuf,
    },
    /// Emitted when the build process is generating code.
    Generating {
        /// The number of blocks being used to generate.
        block_count: usize,
    },
    /// Emitted when ephemeral artifacts are being cleaned up after generation.
    CleaningUp {
        /// The number of ephemeral paths to be removed.
        path_count: usize,
    },
    /// Emitted when an ephemeral artifact path could not be removed.
    CleanupRemoveFailed {
        /// The path that could not be removed.
        path: PathBuf,
        /// The error message describing why removal failed.
        error: String,
    },
    /// Emitted when ephemeral artifact cleanup has completed.
    CleanupComplete,
    /// Emitted when the generation process has completed successfully.
    Success {
        /// The virtual file system containing the generated files.
        vfs: VirtualFileSystem,
    },
    /// Emitted when the compilation process has failed.
    Failure {
        /// The error message describing the failure.
        error: String,
    },
}

impl GenerateEvent {
    pub fn kind(&self) -> &'static str {
        match self {
            GenerateEvent::GenerateStarted { .. } => "GenerateStarted",
            GenerateEvent::FetchingSources => "FetchingSources",
            GenerateEvent::SourcesFetched { .. } => "SourcesFetched",
            GenerateEvent::TransformingAssets { .. } => "TransformingAssets",
            GenerateEvent::TransformerStdout(_) => "TransformerStdout",
            GenerateEvent::TransformerStderr(_) => "TransformerStderr",
            GenerateEvent::AssetsTransformed { .. } => "AssetsTransformed",
            GenerateEvent::ResolvingManifest => "ResolvingManifest",
            GenerateEvent::ManifestResolved { .. } => "ManifestResolved",
            GenerateEvent::Composing => "Composing",
            GenerateEvent::Composed { .. } => "Composed",
            GenerateEvent::ExtractingRuntime { .. } => "ExtractingRuntime",
            GenerateEvent::RuntimeExtracted { .. } => "RuntimeExtracted",
            GenerateEvent::Generating { .. } => "Generating",
            GenerateEvent::CleaningUp { .. } => "CleaningUp",
            GenerateEvent::CleanupRemoveFailed { .. } => "CleanupRemoveFailed",
            GenerateEvent::CleanupComplete => "CleanupComplete",
            GenerateEvent::Success { .. } => "Success",
            GenerateEvent::Failure { .. } => "Failure",
        }
    }
}

impl From<FetchStepEvent> for GenerateEvent {
    fn from(event: FetchStepEvent) -> Self {
        match event {
            FetchStepEvent::Started => GenerateEvent::FetchingSources,
            FetchStepEvent::Completed { count } => GenerateEvent::SourcesFetched { count },
        }
    }
}

impl From<TransformStepEvent> for GenerateEvent {
    fn from(event: TransformStepEvent) -> Self {
        match event {
            TransformStepEvent::Started { count } => GenerateEvent::TransformingAssets { count },
            TransformStepEvent::TransformerStdout(line) => GenerateEvent::TransformerStdout(line),
            TransformStepEvent::TransformerStderr(line) => GenerateEvent::TransformerStderr(line),
            TransformStepEvent::Completed { count } => GenerateEvent::AssetsTransformed { count },
        }
    }
}

impl From<ResolveStepEvent> for GenerateEvent {
    fn from(event: ResolveStepEvent) -> Self {
        match event {
            ResolveStepEvent::Started => GenerateEvent::ResolvingManifest,
            ResolveStepEvent::Completed {
                tool_count,
                block_count,
                agent_name,
                archetype_name,
            } => GenerateEvent::ManifestResolved {
                agent_name,
                archetype_name,
                tool_count,
                block_count,
            },
        }
    }
}

impl From<ComposeStepEvent> for GenerateEvent {
    fn from(event: ComposeStepEvent) -> Self {
        match event {
            ComposeStepEvent::Started => GenerateEvent::Composing,
            ComposeStepEvent::Completed {
                archetype_name,
                graph_name,
                protocol_names,
                block_count,
            } => GenerateEvent::Composed {
                archetype_name,
                graph_name,
                protocol_names,
                block_count,
            },
        }
    }
}

impl From<GenerateStepEvent> for GenerateEvent {
    fn from(event: GenerateStepEvent) -> Self {
        match event {
            GenerateStepEvent::Started { block_count } => GenerateEvent::Generating { block_count },
            GenerateStepEvent::Completed { vfs } => GenerateEvent::Success { vfs },
        }
    }
}

impl From<ExtractStepEvent> for GenerateEvent {
    fn from(event: ExtractStepEvent) -> Self {
        match event {
            ExtractStepEvent::Extracting { asset_count } => {
                GenerateEvent::ExtractingRuntime { asset_count }
            }
            ExtractStepEvent::Extracted { runtime_dir } => {
                GenerateEvent::RuntimeExtracted { runtime_dir }
            }
        }
    }
}

impl From<CleanupStepEvent> for GenerateEvent {
    fn from(event: CleanupStepEvent) -> Self {
        match event {
            CleanupStepEvent::Started { path_count } => GenerateEvent::CleaningUp { path_count },
            CleanupStepEvent::RemoveFailed { path, error } => {
                GenerateEvent::CleanupRemoveFailed { path, error }
            }
            CleanupStepEvent::Completed => GenerateEvent::CleanupComplete,
        }
    }
}

/// The successful result of a completed [`GeneratePipeline`](crate::generate::pipeline::GeneratePipeline) run.
#[derive(Debug, Clone)]
pub struct GenerateResult {
    /// The name of the agent being generated.
    pub agent_name: String,
    /// The name of the archetype being used for generation.
    pub archetype_name: String,
    /// The name of the graph being used for generation.
    pub graph_name: String,
    /// The virtual file system containing the generated files.
    pub vfs: VirtualFileSystem,
}
