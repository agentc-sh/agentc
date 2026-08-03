// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

use crate::pipeline::steps::{
    cleanup::CleanupStepEvent, compile::CompileStepEvent, compose::ComposeStepEvent,
    extract::ExtractStepEvent, fetch::FetchStepEvent, generate::GenerateStepEvent,
    launch::LaunchStepEvent, preflight::PreflightStepEvent, resolve::ResolveStepEvent,
    transform::TransformStepEvent,
};

/// Parameters for configuring a run in the [`RunPipeline`](crate::run::pipeline::RunPipeline).
#[derive(Debug, Clone)]
pub struct RunParams {
    pub context_dir: PathBuf,
    pub output_dir: PathBuf,
    pub target_dir: PathBuf,
    pub runtime_dir: PathBuf,
    pub cache_dir: Option<PathBuf>,
    pub no_cache: bool,
    pub release: bool,
    pub verbose: bool,
    pub build_args: Vec<String>,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RunEvent {
    RunStarted {
        /// The name of the agent being run, as defined in the manifest.
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
    /// Emitted when the composition is being checked against the run preconditions.
    Preflighting,
    /// Emitted when every run precondition has passed.
    PreflightPassed,
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
    /// Emitted when the run is generating code.
    Generating {
        /// The number of blocks being used to generate.
        block_count: usize,
    },
    /// Emitted when code generation is complete.
    GenerationComplete {
        /// The number of files generated.
        file_count: usize,
    },
    /// Emitted when the run is writing files to disk.
    Writing {
        /// The directory where the generated files will be written.
        project_dir: PathBuf,
    },
    /// Emitted when the run has finished writing files to disk.
    WriteComplete {
        /// The directory where the generated files were written.
        project_dir: PathBuf,
    },
    /// Emitted when the run is compiling the generated code.
    Compiling {
        /// Whether the compilation is a release build or a debug build.
        release: bool,
    },
    /// Emitted when stdout is received from the compiler.
    CompilerStdout(String),
    /// Emitted when stderr is received from the compiler.
    CompilerStderr(String),
    /// Emitted when the compilation has completed successfully.
    Compiled {
        /// The directory where the compiled artifact was written.
        output_dir: PathBuf,
    },
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
    /// Emitted when the built artifact is about to be invoked.
    Launching,
    /// Emitted when the invocation has finished.
    Exited {
        /// The status the invocation reported, if any.
        exit_code: Option<i32>,
    },
    /// Emitted when the run has failed.
    Failure {
        /// The error message describing the failure.
        error: String,
    },
}

impl RunEvent {
    /// Returns true for the two terminal events [`Exited`](Self::Exited)
    /// and [`Failure`](Self::Failure), and false for all other events.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Exited { .. } | Self::Failure { .. })
    }
}

impl From<CleanupStepEvent> for RunEvent {
    fn from(event: CleanupStepEvent) -> Self {
        match event {
            CleanupStepEvent::Started { path_count } => RunEvent::CleaningUp { path_count },
            CleanupStepEvent::RemoveFailed { path, error } => {
                RunEvent::CleanupRemoveFailed { path, error }
            }
            CleanupStepEvent::Completed => RunEvent::CleanupComplete,
        }
    }
}

impl From<FetchStepEvent> for RunEvent {
    fn from(event: FetchStepEvent) -> Self {
        match event {
            FetchStepEvent::Started => RunEvent::FetchingSources,
            FetchStepEvent::Completed { count } => RunEvent::SourcesFetched { count },
        }
    }
}

impl From<TransformStepEvent> for RunEvent {
    fn from(event: TransformStepEvent) -> Self {
        match event {
            TransformStepEvent::Started { count } => RunEvent::TransformingAssets { count },
            TransformStepEvent::TransformerStdout(line) => RunEvent::TransformerStdout(line),
            TransformStepEvent::TransformerStderr(line) => RunEvent::TransformerStderr(line),
            TransformStepEvent::Completed { count } => RunEvent::AssetsTransformed { count },
        }
    }
}

impl From<ResolveStepEvent> for RunEvent {
    fn from(event: ResolveStepEvent) -> Self {
        match event {
            ResolveStepEvent::Started => RunEvent::ResolvingManifest,
            ResolveStepEvent::Completed {
                tool_count,
                block_count,
                agent_name,
                archetype_name,
            } => RunEvent::ManifestResolved {
                agent_name,
                archetype_name,
                tool_count,
                block_count,
            },
        }
    }
}

impl From<ComposeStepEvent> for RunEvent {
    fn from(event: ComposeStepEvent) -> Self {
        match event {
            ComposeStepEvent::Started => RunEvent::Composing,
            ComposeStepEvent::Completed {
                archetype_name,
                graph_name,
                protocol_names,
                block_count,
            } => RunEvent::Composed {
                archetype_name,
                graph_name,
                protocol_names,
                block_count,
            },
        }
    }
}

impl From<PreflightStepEvent> for RunEvent {
    fn from(event: PreflightStepEvent) -> Self {
        match event {
            PreflightStepEvent::Started { .. } => RunEvent::Preflighting,
            PreflightStepEvent::Completed => RunEvent::PreflightPassed,
        }
    }
}

impl From<GenerateStepEvent> for RunEvent {
    fn from(event: GenerateStepEvent) -> Self {
        match event {
            GenerateStepEvent::Started { block_count } => RunEvent::Generating { block_count },
            GenerateStepEvent::Completed { vfs } => {
                RunEvent::GenerationComplete { file_count: vfs.len() }
            }
        }
    }
}

impl From<ExtractStepEvent> for RunEvent {
    fn from(event: ExtractStepEvent) -> Self {
        match event {
            ExtractStepEvent::Extracting { asset_count } => {
                RunEvent::ExtractingRuntime { asset_count }
            }
            ExtractStepEvent::Extracted { runtime_dir } => {
                RunEvent::RuntimeExtracted { runtime_dir }
            }
        }
    }
}

impl From<CompileStepEvent> for RunEvent {
    fn from(event: CompileStepEvent) -> Self {
        match event {
            CompileStepEvent::WritingFiles { project_dir } => RunEvent::Writing { project_dir },
            CompileStepEvent::WriteCompleted { project_dir } => {
                RunEvent::WriteComplete { project_dir }
            }
            CompileStepEvent::Compiling { release } => RunEvent::Compiling { release },
            CompileStepEvent::CompilerStdout(line) => RunEvent::CompilerStdout(line),
            CompileStepEvent::CompilerStderr(line) => RunEvent::CompilerStderr(line),
            CompileStepEvent::CompileCompleted { output_dir } => RunEvent::Compiled { output_dir },
        }
    }
}

impl From<LaunchStepEvent> for RunEvent {
    fn from(event: LaunchStepEvent) -> Self {
        match event {
            LaunchStepEvent::Launching => RunEvent::Launching,
            LaunchStepEvent::Exited { exit_code } => RunEvent::Exited { exit_code },
        }
    }
}

/// The successful result of a completed [`RunPipeline`](crate::run::pipeline::RunPipeline) run.
#[derive(Debug, Clone)]
pub struct RunResult {
    /// The status the invocation reported, if any.
    pub exit_code: Option<i32>,
}
