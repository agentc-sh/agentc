// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use guestjs::errors::BorrowKind;

#[derive(Debug, thiserror::Error)]
pub enum GuestError {
    #[error("unexpected error: {message}")]
    Unexpected {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("guest exception: {name}: {message}")]
    Guest {
        name: String,
        message: String,
        stack: Option<String>,
    },

    #[error("expected {expected}, found {found}")]
    Conversion { expected: String, found: String },

    #[error("a value was used outside the guest it belongs to")]
    ForeignContext,

    #[error("{type_name} is already borrowed ({kind})")]
    Borrow { kind: BorrowKind, type_name: String },

    #[error("{message}")]
    Unsupported { message: String },

    #[error("execution was interrupted")]
    Interrupted,

    #[error("execution timed out")]
    Timeout,

    #[error("execution was cancelled")]
    Cancelled,

    #[error("the guest is closed")]
    Closed,

    #[error("{message}")]
    Engine { message: String },

    #[error("cannot import {name}: {message}")]
    Import { name: String, message: String },

    #[error("invalid bundle: {message}")]
    Bundle { message: String },

    #[error(transparent)]
    Host(Box<dyn std::error::Error + Send + Sync>),
}

impl From<guestjs::errors::Error> for GuestError {
    fn from(error: guestjs::errors::Error) -> Self {
        match error {
            guestjs::errors::Error::Unexpected { message, source } => {
                Self::Unexpected { message, source }
            }
            guestjs::errors::Error::Guest(exception) => Self::Guest {
                name: exception.name().to_owned(),
                message: exception.message().to_owned(),
                stack: exception.stack().map(str::to_owned),
            },
            guestjs::errors::Error::Conversion { expected, found } => {
                Self::Conversion { expected, found }
            }
            guestjs::errors::Error::ForeignContext => Self::ForeignContext,
            guestjs::errors::Error::Borrow { kind, type_name } => Self::Borrow { kind, type_name },
            guestjs::errors::Error::Unsupported { message } => Self::Unsupported { message },
            guestjs::errors::Error::Interrupted => Self::Interrupted,
            guestjs::errors::Error::Timeout => Self::Timeout,
            guestjs::errors::Error::Cancelled => Self::Cancelled,
            guestjs::errors::Error::Closed => Self::Closed,
            guestjs::errors::Error::Engine { message } => Self::Engine { message },
            guestjs::errors::Error::Import { name, message } => Self::Import { name, message },
            guestjs::errors::Error::Bundle { message } => Self::Bundle { message },
            guestjs::errors::Error::Host(error) => Self::Host(error),
        }
    }
}

/// Errors produced while constructing, operating, or shutting down an executor.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An operation failed inside GuestJS.
    #[error(transparent)]
    Guest(#[from] GuestError),

    /// No Tokio runtime was active where the host runtime was captured.
    #[error("no active runtime to capture as the host runtime")]
    NoHostRuntime,

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

    #[error("runtime configuration failed: {0}")]
    Configuration(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// A worker could not initialize its GuestJS package environment.
    #[error("failed to initialize worker {worker}: {source}")]
    WorkerInitialization {
        worker: usize,
        #[source]
        source: Box<Error>,
    },

    /// A worker stopped accepting submitted work.
    #[error("worker {worker} is unavailable")]
    WorkerUnavailable { worker: usize },

    /// A worker exited before returning an execution result.
    #[error("worker {worker} dropped its execution response")]
    WorkerResponseDropped { worker: usize },

    #[error("failed to shut down worker {worker}: {source}")]
    WorkerShutdown {
        worker: usize,
        #[source]
        source: GuestError,
    },

    /// Work was submitted after executor shutdown began.
    #[error("executor is shut down")]
    ExecutorShutdown,

    /// Executor shutdown was requested from one of its own workers.
    #[error("executor cannot shut down from one of its own workers")]
    ReentrantShutdown,

    /// A worker panicked while shutting down.
    #[error("worker {worker} panicked")]
    WorkerPanicked { worker: usize },

    /// The host runtime stopped or the offloaded task panicked before returning a result.
    #[error("the host runtime did not return a result for the offloaded future")]
    HostRuntimeStopped,

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
    pub fn guest(error: impl Into<GuestError>) -> Self {
        Self::Guest(error.into())
    }

    /// Creates an [`Error::NoHostRuntime`] error.
    pub fn no_host_runtime() -> Self {
        Self::NoHostRuntime
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

    pub fn configuration(source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Configuration(source.into())
    }

    /// Creates an [`Error::WorkerInitialization`] error.
    pub fn worker_initialization(worker: impl Into<usize>, source: impl Into<Error>) -> Self {
        Self::WorkerInitialization {
            worker: worker.into(),
            source: Box::new(source.into()),
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

    pub fn worker_shutdown(worker: impl Into<usize>, source: impl Into<GuestError>) -> Self {
        Self::WorkerShutdown {
            worker: worker.into(),
            source: source.into(),
        }
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

    /// Creates an [`Error::HostRuntimeStopped`] error.
    pub fn host_runtime_stopped() -> Self {
        Self::HostRuntimeStopped
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

impl From<guestjs::errors::Error> for Error {
    fn from(error: guestjs::errors::Error) -> Self {
        Self::Guest(GuestError::from(error))
    }
}
