// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

/// Errors produced while constructing, operating, or shutting down an executor.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An operation failed inside GuestJS.
    #[error(transparent)]
    Guest(#[from] guestjs::errors::Error),

    /// The executor was configured without any workers.
    #[error("executor worker count must be greater than zero")]
    InvalidWorkerCount,

    /// The executor was configured with a zero-capacity worker queue.
    #[error("executor queue capacity must be greater than zero")]
    InvalidQueueCapacity,

    /// A worker thread could not be spawned.
    #[error("failed to spawn worker {worker}: {source}")]
    WorkerSpawn {
        worker: usize,
        #[source]
        source: std::io::Error,
    },

    /// A worker could not construct its Tokio runtime.
    #[error("failed to build runtime for worker {worker}: {source}")]
    WorkerRuntime {
        worker: usize,
        #[source]
        source: std::io::Error,
    },

    /// A worker could not initialize its GuestJS package environment.
    #[error("failed to initialize worker {worker}: {source}")]
    WorkerInitialization {
        worker: usize,
        #[source]
        source: guestjs::errors::Error,
    },

    /// A worker stopped accepting submitted work.
    #[error("worker {worker} is unavailable")]
    WorkerUnavailable { worker: usize },

    /// A worker exited before returning an execution result.
    #[error("worker {worker} dropped its execution response")]
    WorkerResponseDropped { worker: usize },

    /// Work was submitted after executor shutdown began.
    #[error("executor is shut down")]
    ExecutorShutdown,

    /// Executor shutdown was requested from one of its own workers.
    #[error("executor cannot shut down from one of its own workers")]
    ReentrantShutdown,

    /// A worker panicked while shutting down.
    #[error("worker {worker} panicked")]
    WorkerPanicked { worker: usize },

    /// The blocking worker-join task failed.
    #[error("worker join task failed: {0}")]
    JoinTask(#[from] tokio::task::JoinError),

    /// An unexpected error occurred.
    #[error("unexpected error: {message}")]
    Unexpected {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl Error {
    /// Creates an [`Error::Guest`] error.
    pub fn guest(error: impl Into<guestjs::errors::Error>) -> Self {
        Self::Guest(error.into())
    }

    /// Creates an [`Error::InvalidWorkerCount`] error.
    pub fn invalid_worker_count() -> Self {
        Self::InvalidWorkerCount
    }

    /// Creates an [`Error::InvalidQueueCapacity`] error.
    pub fn invalid_queue_capacity() -> Self {
        Self::InvalidQueueCapacity
    }

    /// Creates an [`Error::WorkerSpawn`] error.
    pub fn worker_spawn(worker: impl Into<usize>, source: impl Into<std::io::Error>) -> Self {
        Self::WorkerSpawn {
            worker: worker.into(),
            source: source.into(),
        }
    }

    /// Creates an [`Error::WorkerRuntime`] error.
    pub fn worker_runtime(worker: impl Into<usize>, source: impl Into<std::io::Error>) -> Self {
        Self::WorkerRuntime {
            worker: worker.into(),
            source: source.into(),
        }
    }

    /// Creates an [`Error::WorkerInitialization`] error.
    pub fn worker_initialization(
        worker: impl Into<usize>,
        source: impl Into<guestjs::errors::Error>,
    ) -> Self {
        Self::WorkerInitialization {
            worker: worker.into(),
            source: source.into(),
        }
    }

    /// Creates an [`Error::WorkerUnavailable`] error.
    pub fn worker_unavailable(worker: impl Into<usize>) -> Self {
        Self::WorkerUnavailable { worker: worker.into() }
    }

    /// Creates an [`Error::WorkerResponseDropped`] error.
    pub fn worker_response_dropped(worker: impl Into<usize>) -> Self {
        Self::WorkerResponseDropped { worker: worker.into() }
    }

    /// Creates an [`Error::ExecutorShutdown`] error.
    pub fn executor_shutdown() -> Self {
        Self::ExecutorShutdown
    }

    /// Creates an [`Error::ReentrantShutdown`] error.
    pub fn reentrant_shutdown() -> Self {
        Self::ReentrantShutdown
    }

    /// Creates an [`Error::WorkerPanicked`] error.
    pub fn worker_panicked(worker: impl Into<usize>) -> Self {
        Self::WorkerPanicked { worker: worker.into() }
    }

    /// Creates an [`Error::JoinTask`] error.
    pub fn join_task(error: impl Into<tokio::task::JoinError>) -> Self {
        Self::JoinTask(error.into())
    }

    /// Creates an [`Error::Unexpected`] error.
    pub fn unexpected(
        message: impl Into<String>,
        source: impl Into<Option<Box<dyn std::error::Error + Send + Sync>>>,
    ) -> Self {
        Self::Unexpected {
            message: message.into(),
            source: source.into(),
        }
    }
}
