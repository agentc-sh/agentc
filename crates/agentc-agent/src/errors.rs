// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

use crate::graph::errors::GraphError;

#[derive(Error, Debug)]
pub enum AgentError {
    /// A configuration error such as missing fields or invalid values in the agent configuration.
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// An error that occurs during the execution of the agent graph.
    #[error("Graph error: {0}")]
    Graph(#[from] GraphError),
}

impl AgentError {
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration { message: message.into() }
    }

    pub fn graph(error: GraphError) -> Self {
        Self::Graph(error)
    }
}
