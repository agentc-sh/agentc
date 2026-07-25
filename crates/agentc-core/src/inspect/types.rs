// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_blocks::context::ResolvedContext;

use crate::pipeline::steps::{
    compose::ComposeStepEvent, fetch::FetchStepEvent, resolve::ResolveStepEvent,
    transform::TransformStepEvent,
};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum InspectEvent {
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
        /// The total number of blocks that would be rendered.
        block_count: usize,
    },
    /// Emitted when the resolution process has completed successfully.
    Success {
        /// The name of the agent being generated.
        agent_name: String,
        /// The name of the archetype being used for generation.
        archetype_name: String,
        /// The name of the graph being used for generation.
        graph_name: String,
        /// Resolved context containing all information about the agent and archetype configuration.
        context: Box<ResolvedContext>,
    },
    /// Emitted when the inspection process has failed.
    Failure {
        /// The error message describing the failure.
        error: String,
    },
}

impl InspectEvent {
    pub fn kind(&self) -> &'static str {
        match self {
            InspectEvent::FetchingSources => "FetchingSources",
            InspectEvent::SourcesFetched { .. } => "SourcesFetched",
            InspectEvent::TransformingAssets { .. } => "TransformingAssets",
            InspectEvent::TransformerStdout(_) => "TransformerStdout",
            InspectEvent::TransformerStderr(_) => "TransformerStderr",
            InspectEvent::AssetsTransformed { .. } => "AssetsTransformed",
            InspectEvent::ResolvingManifest => "ResolvingManifest",
            InspectEvent::ManifestResolved { .. } => "ManifestResolved",
            InspectEvent::Composing => "Composing",
            InspectEvent::Composed { .. } => "Composed",
            InspectEvent::Success { .. } => "Success",
            InspectEvent::Failure { .. } => "Failure",
        }
    }
}

impl From<FetchStepEvent> for InspectEvent {
    fn from(event: FetchStepEvent) -> Self {
        match event {
            FetchStepEvent::Started => InspectEvent::FetchingSources,
            FetchStepEvent::Completed { count } => InspectEvent::SourcesFetched { count },
        }
    }
}

impl From<TransformStepEvent> for InspectEvent {
    fn from(event: TransformStepEvent) -> Self {
        match event {
            TransformStepEvent::Started { count } => InspectEvent::TransformingAssets { count },
            TransformStepEvent::TransformerStdout(line) => InspectEvent::TransformerStdout(line),
            TransformStepEvent::TransformerStderr(line) => InspectEvent::TransformerStderr(line),
            TransformStepEvent::Completed { count } => InspectEvent::AssetsTransformed { count },
        }
    }
}

impl From<ResolveStepEvent> for InspectEvent {
    fn from(event: ResolveStepEvent) -> Self {
        match event {
            ResolveStepEvent::Started => InspectEvent::ResolvingManifest,
            ResolveStepEvent::Completed {
                tool_count,
                block_count,
                agent_name,
                archetype_name,
            } => InspectEvent::ManifestResolved {
                agent_name,
                archetype_name,
                tool_count,
                block_count,
            },
        }
    }
}

impl From<ComposeStepEvent> for InspectEvent {
    fn from(event: ComposeStepEvent) -> Self {
        match event {
            ComposeStepEvent::Started => InspectEvent::Composing,
            ComposeStepEvent::Completed {
                archetype_name,
                graph_name,
                protocol_names,
                block_count,
            } => InspectEvent::Composed {
                archetype_name,
                graph_name,
                protocol_names,
                block_count,
            },
        }
    }
}

/// The successful result of a completed [`InspectPipeline`](crate::inspect::pipeline::InspectPipeline) run.
#[derive(Debug, Clone)]
pub struct InspectResult {
    /// The name of the agent being inspected.
    pub agent_name: String,
    /// The name of the archetype being used for inspection.
    pub archetype_name: String,
    /// The name of the graph being used for inspection.
    pub graph_name: String,
    /// The names of the protocols being used for inspection, in manifest order.
    pub protocol_names: Vec<String>,
    /// The resolved context containing all information about the agent and archetype configuration.
    pub context: ResolvedContext,
}
