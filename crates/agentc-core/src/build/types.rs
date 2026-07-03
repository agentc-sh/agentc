// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

use crate::pipeline::steps::{
    cleanup::CleanupStepEvent, compile::CompileStepEvent, extract::ExtractStepEvent,
    fetch::FetchStepEvent, generate::GenerateStepEvent, resolve::ResolveStepEvent,
    transform::TransformStepEvent,
};

/// Parameters for configuring the build process in the [`BuildPipeline`](crate::build::pipeline::BuildPipeline).
#[derive(Debug, Clone)]
pub struct BuildParams {
    pub output_dir: PathBuf,
    pub target_dir: PathBuf,
    pub runtime_dir: PathBuf,
    pub cache_dir: Option<PathBuf>,
    pub no_cache: bool,
    pub release: bool,
    pub verbose: bool,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum BuildEvent {
    BuildStarted {
        /// The name of the agent being built, as defined in the manifest.
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
    /// Emitted when code generation is complete.
    GenerationComplete {
        /// The number of files generated.
        file_count: usize,
    },
    /// Emitted when the build process is writing files to disk.
    Writing {
        /// The directory where the generated files will be written.
        project_dir: PathBuf,
    },
    /// Emitted when the build process has finished writing files to disk.
    WriteComplete {
        /// The directory where the generated files were written.
        project_dir: PathBuf,
    },
    /// Emitted when the build process is compiling the generated code.
    Compiling {
        /// Whether the compilation is a release build or a debug build.
        release: bool,
    },
    /// Emitted when stdout is received from the compiler.
    CompilerStdout(String),
    /// Emitted when stderr is received from the compiler.
    CompilerStderr(String),
    /// Emitted when ephemeral artifacts are being cleaned up after compilation.
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
    /// Emitted when the compilation process has completed successfully.
    Success {
        /// The directory where the compiled artifact was written.
        output_dir: PathBuf,
    },
    /// Emitted when the compilation process has failed.
    Failure {
        /// The error message describing the failure.
        error: String,
    },
}

impl BuildEvent {
    /// Returns true for the two terminal events [`Success`](Self::Success)
    /// and [`Failure`](Self::Failure), and false for all other events.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Success { .. } | Self::Failure { .. })
    }
}

impl From<CleanupStepEvent> for BuildEvent {
    fn from(event: CleanupStepEvent) -> Self {
        match event {
            CleanupStepEvent::Started { path_count } => BuildEvent::CleaningUp { path_count },
            CleanupStepEvent::RemoveFailed { path, error } => {
                BuildEvent::CleanupRemoveFailed { path, error }
            }
            CleanupStepEvent::Completed => BuildEvent::CleanupComplete,
        }
    }
}

impl From<FetchStepEvent> for BuildEvent {
    fn from(event: FetchStepEvent) -> Self {
        match event {
            FetchStepEvent::Started => BuildEvent::FetchingSources,
            FetchStepEvent::Completed { count } => BuildEvent::SourcesFetched { count },
        }
    }
}

impl From<TransformStepEvent> for BuildEvent {
    fn from(event: TransformStepEvent) -> Self {
        match event {
            TransformStepEvent::Started { count } => BuildEvent::TransformingAssets { count },
            TransformStepEvent::TransformerStdout(line) => BuildEvent::TransformerStdout(line),
            TransformStepEvent::TransformerStderr(line) => BuildEvent::TransformerStderr(line),
            TransformStepEvent::Completed { count } => BuildEvent::AssetsTransformed { count },
        }
    }
}

impl From<ResolveStepEvent> for BuildEvent {
    fn from(event: ResolveStepEvent) -> Self {
        match event {
            ResolveStepEvent::Started => BuildEvent::ResolvingManifest,
            ResolveStepEvent::Completed {
                tool_count,
                block_count,
                agent_name,
                archetype_name,
            } => BuildEvent::ManifestResolved {
                agent_name,
                archetype_name,
                tool_count,
                block_count,
            },
        }
    }
}

impl From<GenerateStepEvent> for BuildEvent {
    fn from(event: GenerateStepEvent) -> Self {
        match event {
            GenerateStepEvent::Started { block_count } => BuildEvent::Generating { block_count },
            GenerateStepEvent::Completed { vfs } => {
                BuildEvent::GenerationComplete { file_count: vfs.len() }
            }
        }
    }
}

impl From<ExtractStepEvent> for BuildEvent {
    fn from(event: ExtractStepEvent) -> Self {
        match event {
            ExtractStepEvent::Extracting { asset_count } => {
                BuildEvent::ExtractingRuntime { asset_count }
            }
            ExtractStepEvent::Extracted { runtime_dir } => {
                BuildEvent::RuntimeExtracted { runtime_dir }
            }
        }
    }
}

impl From<CompileStepEvent> for BuildEvent {
    fn from(event: CompileStepEvent) -> Self {
        match event {
            CompileStepEvent::WritingFiles { project_dir } => BuildEvent::Writing { project_dir },
            CompileStepEvent::WriteCompleted { project_dir } => {
                BuildEvent::WriteComplete { project_dir }
            }
            CompileStepEvent::Compiling { release } => BuildEvent::Compiling { release },
            CompileStepEvent::CompilerStdout(line) => BuildEvent::CompilerStdout(line),
            CompileStepEvent::CompilerStderr(line) => BuildEvent::CompilerStderr(line),
            CompileStepEvent::CompileCompleted { output_dir } => BuildEvent::Success { output_dir },
        }
    }
}

/// The successful result of a completed [`BuildPipeline`](crate::build::pipeline::BuildPipeline) run.
#[derive(Debug, Clone)]
pub struct BuildResult {
    /// The directory on disk where the compiled artifact was written.
    pub artifact_dir: PathBuf,
}
