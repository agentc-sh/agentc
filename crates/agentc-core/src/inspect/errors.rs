// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

use crate::{
    manifest::errors::ManifestError,
    pipeline::steps::{
        fetch::FetchStepError, resolve::ResolveStepError, transform::TransformStepError,
    },
};

#[derive(Error, Debug)]
pub enum InspectError {
    #[error("pipeline configuration error: {0}")]
    PipelineConfiguration(String),

    #[error("manifest error: {0}")]
    Manifest(#[from] ManifestError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("fetch step error: {0}")]
    Fetch(#[from] FetchStepError),

    #[error("transform step error: {0}")]
    Transform(#[from] TransformStepError),

    #[error("resolve step error: {0}")]
    Resolve(#[from] ResolveStepError),

    #[error("build event receiver was dropped before pipeline finished")]
    EventChannelClosed,
}

impl InspectError {
    pub fn pipeline_configuration(message: impl Into<String>) -> Self {
        Self::PipelineConfiguration(message.into())
    }

    pub fn event_channel_closed() -> Self {
        Self::EventChannelClosed
    }
}
