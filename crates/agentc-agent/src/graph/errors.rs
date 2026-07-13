// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

use crate::graph::checkpoint::errors::CheckpointError;

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("node not found in graph: {0:?}")]
    NodeNotFound(String),

    #[error("checkpoint error: {0}")]
    CheckpointError(#[from] CheckpointError),

    /// Signals that a node has requested an interruption. This is not a failure.
    /// [`Graph::run`](crate::graph::graph::Graph::run) catches this variant and ends the run
    /// cleanly with [`RunStatus::Interrupted`](crate::graph::checkpoint::types::RunStatus::Interrupted).
    #[error("graph interrupted")]
    Interrupt(serde_json::Value),

    #[error("graph execution error: {message}")]
    ExecutionError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("graph state conversion error: {message}")]
    ConversionError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("cancellation error: {0}")]
    Cancellation(String),
}

impl GraphError {
    pub fn error_type(&self) -> &'static str {
        match self {
            Self::NodeNotFound(_) => "node_not_found",
            Self::CheckpointError(_) => "checkpoint",
            Self::Interrupt(_) => "interrupt",
            Self::ExecutionError { .. } => "execution",
            Self::ConversionError { .. } => "conversion",
            Self::Cancellation(_) => "cancellation",
        }
    }

    pub fn node_not_found(node: impl ToString) -> Self {
        Self::NodeNotFound(node.to_string())
    }

    pub fn checkpoint_error(source: CheckpointError) -> Self {
        Self::CheckpointError(source)
    }

    pub fn execution_error<E>(source: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        let source = source.into();

        Self::ExecutionError {
            message: source.to_string(),
            source: Some(source),
        }
    }

    pub fn execution_error_message(message: impl Into<String>) -> Self {
        Self::ExecutionError { message: message.into(), source: None }
    }

    pub fn conversion_error<E>(source: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        let source = source.into();

        Self::ConversionError {
            message: source.to_string(),
            source: Some(source),
        }
    }

    pub fn conversion_error_message(message: impl Into<String>) -> Self {
        Self::ConversionError { message: message.into(), source: None }
    }

    pub fn cancellation_error(message: impl Into<String>) -> Self {
        Self::Cancellation(message.into())
    }
}

impl From<anyhow::Error> for GraphError {
    fn from(err: anyhow::Error) -> Self {
        GraphError::execution_error(err)
    }
}

impl From<String> for GraphError {
    fn from(err: String) -> Self {
        GraphError::execution_error_message(err)
    }
}
