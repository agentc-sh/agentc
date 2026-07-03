// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

use agentc_agent::tools::errors::ToolError;

#[derive(Debug, Error)]
pub enum RuntimeError {
    /// Errors related to the QuickJS runtime at initialization.
    #[error("failed to initialize QuickJS runtime: {0}")]
    Init(rquickjs::Error),

    /// Errors related to loading a JavaScript/TypeScript module.
    #[error("failed to load module: {0}")]
    ModuleLoad(rquickjs::Error),

    /// Errors that occur during JavaScript execution (e.g. syntax errors, runtime exceptions).
    #[error("javascript error: {message}")]
    Js {
        #[source]
        source: rquickjs::Error,
        message: String,
    },

    /// Errors that occur during serialization/deserialization of arguments or results.
    #[error("serialization error: {0}")]
    Serialize(serde_json::Error),

    /// Error that occurs when the worker thread has closed.
    #[error("worker thread has closed")]
    WorkerClosed,
}

impl RuntimeError {
    pub fn init(err: rquickjs::Error) -> Self {
        RuntimeError::Init(err)
    }

    pub fn module_load(err: rquickjs::Error) -> Self {
        RuntimeError::ModuleLoad(err)
    }

    pub fn js(err: rquickjs::Error) -> Self {
        RuntimeError::Js { message: err.to_string(), source: err }
    }

    pub fn js_with_message(source: rquickjs::Error, message: String) -> Self {
        RuntimeError::Js { source, message }
    }

    pub fn serialize(err: serde_json::Error) -> Self {
        RuntimeError::Serialize(err)
    }

    pub fn worker_closed() -> Self {
        RuntimeError::WorkerClosed
    }
}

impl From<RuntimeError> for ToolError {
    fn from(err: RuntimeError) -> Self {
        ToolError::sourced_execution_error("javascript", err.to_string(), Some(err))
    }
}
