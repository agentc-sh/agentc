// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

use agentc_agent::tools::errors::ToolError;

#[derive(Debug, Error)]
pub enum RuntimeError {
    /// An I/O error occurred in the worker thread (e.g. channel send failed).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// The interpreter failed to initialize (frozen stdlib load, module eval, etc.).
    #[error("failed to initialize Python interpreter: {0}")]
    Init(String),

    /// The tool package failed to load or a named class could not be found.
    #[error("failed to load Python tool: {0}")]
    ToolLoad(String),

    /// A Python exception was raised during execution.
    #[error("python error: {0}")]
    Python(String),

    /// JSON serialization or deserialization of arguments or results failed.
    #[error("serialization error: {0}")]
    Serialize(#[from] serde_json::Error),

    /// A timeout elapsed while waiting for a response from the worker thread.
    #[error("operation timed out")]
    Timeout,

    /// The worker thread's inbound channel has closed.
    #[error("worker thread has closed")]
    WorkerClosed,

    /// A command was sent to the worker thread, but the reply channel was dropped before a response could be sent.
    #[error("reply channel was dropped before a response could be sent")]
    ReplyChannelDropped,

    /// The channel to the worker thread is full, indicating that the worker is currently busy processing other commands.
    #[error("worker thread is busy")]
    WorkerBusy,

    /// Failed to close the worker threads during shutdown.
    #[error("failed to close worker thread during shutdown: {0}")]
    Shutdown(String),
}

impl RuntimeError {
    pub fn io(err: std::io::Error) -> Self {
        Self::Io(err)
    }

    pub fn init(msg: impl Into<String>) -> Self {
        Self::Init(msg.into())
    }

    pub fn tool_load(msg: impl Into<String>) -> Self {
        Self::ToolLoad(msg.into())
    }

    pub fn python(msg: impl Into<String>) -> Self {
        Self::Python(msg.into())
    }

    pub fn serialize(err: serde_json::Error) -> Self {
        Self::Serialize(err)
    }

    pub fn timeout() -> Self {
        Self::Timeout
    }

    pub fn worker_closed() -> Self {
        Self::WorkerClosed
    }

    pub fn reply_channel_dropped() -> Self {
        Self::ReplyChannelDropped
    }

    pub fn worker_busy() -> Self {
        Self::WorkerBusy
    }

    pub fn shutdown(msg: impl Into<String>) -> Self {
        Self::Shutdown(msg.into())
    }
}

impl From<RuntimeError> for ToolError {
    fn from(err: RuntimeError) -> Self {
        ToolError::sourced_execution_error("python", err.to_string(), Some(err))
    }
}
