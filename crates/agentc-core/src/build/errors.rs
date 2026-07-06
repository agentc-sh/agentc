// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

use crate::{
    manifest::errors::ManifestError,
    pipeline::steps::{
        cleanup::CleanupStepError, compile::CompileStepError, compose::ComposeStepError,
        extract::ExtractStepError, fetch::FetchStepError, generate::GenerateStepError,
        resolve::ResolveStepError, transform::TransformStepError,
    },
};

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("pipeline configuration error: {0}")]
    PipelineConfiguration(String),

    #[error("manifest error: {0}")]
    Manifest(#[from] ManifestError),

    #[error("fetch step error: {0}")]
    Fetch(#[from] FetchStepError),

    #[error("transform step error: {0}")]
    Transform(#[from] TransformStepError),

    #[error("resolve step error: {0}")]
    Resolve(#[from] ResolveStepError),

    #[error("compose step error: {0}")]
    Compose(#[from] ComposeStepError),

    #[error("generate step error: {0}")]
    Generate(#[from] GenerateStepError),

    #[error("extract step error: {0}")]
    Extract(#[from] ExtractStepError),

    #[error("compile step error: {0}")]
    Compile(#[from] CompileStepError),

    #[error("cleanup step error: {0}")]
    Cleanup(#[from] CleanupStepError),

    #[error("build event receiver was dropped before pipeline finished")]
    EventChannelClosed,
}

impl BuildError {
    pub fn pipeline_configuration(message: impl Into<String>) -> Self {
        Self::PipelineConfiguration(message.into())
    }

    pub fn event_channel_closed() -> Self {
        Self::EventChannelClosed
    }
}
