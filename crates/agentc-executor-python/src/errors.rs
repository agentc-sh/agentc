// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::io;

use guestpy::errors::BorrowKind;
use tokio::task::JoinError;

/// A `Send`-safe mirror of [`guestpy::errors::Error`] with the guest-owned exception
/// object stripped, since it is confined to the interpreter thread that raised it.
#[derive(Debug, thiserror::Error)]
pub enum GuestError {
    /// A Python exception was raised by guest code.
    #[error("guest exception: {qualified_name}: {message}")]
    Guest {
        type_name: String,
        qualified_name: String,
        message: String,
        name: Option<String>,
        traceback: Option<String>,
    },

    #[error(
        "raised {class}{}",
        if .text.is_empty() {
            String::new()
        } else {
            format!(": {}", .text)
        }
    )]
    Raise { class: String, text: String },

    /// A failure originating in the Python engine itself.
    #[error("engine error: {message}")]
    Engine {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// A failure converting a value between Rust and Python.
    #[error("conversion error: {message}")]
    Conversion {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// A named module failed to import.
    #[error("import error: {name}: {message}")]
    Import { name: String, message: String },

    /// A named attribute does not exist on a guest value.
    #[error("no attribute named {name}")]
    Attribute { name: String },

    /// A bundle failed to load at the given path.
    #[error("invalid bundle at {path}: {message}")]
    Bundle { path: String, message: String },

    /// A bundle has more than one top-level module and must be mounted as a library.
    #[error("bundle has {roots} top-level modules; mount it with `library` instead")]
    AmbiguousBundle { roots: usize },

    /// A module name is already loaded in the guest.
    #[error("module {name} is already loaded in this guest")]
    NameInUse { name: String },

    /// A host class is already borrowed.
    #[error("host class {class} is already borrowed ({kind})")]
    Borrow {
        class: &'static str,
        kind: BorrowKind,
    },

    /// The requested operation is not supported by the selected backend.
    #[error("unsupported: {message}")]
    Unsupported { message: String },

    /// A host-defined function or binding failed.
    #[error(transparent)]
    Host(Box<dyn std::error::Error + Send + Sync>),

    /// Execution timed out.
    #[error("execution timed out")]
    Timeout,

    /// Execution was cancelled.
    #[error("execution cancelled")]
    Cancelled,

    /// Execution was interrupted.
    #[error("execution interrupted")]
    Interrupted,

    /// The guest is closed.
    #[error("guest is closed")]
    Closed,

    /// Iteration stopped.
    #[error("iteration stopped")]
    StopIteration,

    /// Async iteration stopped.
    #[error("async iteration stopped")]
    StopAsyncIteration,

    /// An underlying I/O error.
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    /// An unexpected error occurred.
    #[error("unexpected error: {message}")]
    Unexpected {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl From<guestpy::errors::Error> for GuestError {
    fn from(error: guestpy::errors::Error) -> Self {
        match error {
            guestpy::errors::Error::Guest(exception) => Self::Guest {
                type_name: exception.type_name().to_owned(),
                qualified_name: exception.qualified_name().to_owned(),
                message: exception.message().to_owned(),
                name: exception.name().map(str::to_owned),
                traceback: exception.traceback().map(str::to_owned),
            },
            guestpy::errors::Error::Raise(raise) => Self::Raise {
                class: raise.class().to_owned(),
                text: raise.text().to_owned(),
            },
            guestpy::errors::Error::Engine { message, source } => Self::Engine { message, source },
            guestpy::errors::Error::Conversion { message, source } => Self::Conversion { message, source },
            guestpy::errors::Error::Import { name, message } => Self::Import { name, message },
            guestpy::errors::Error::Attribute { name } => Self::Attribute { name },
            guestpy::errors::Error::Bundle { path, message } => Self::Bundle { path, message },
            guestpy::errors::Error::AmbiguousBundle { roots } => Self::AmbiguousBundle { roots },
            guestpy::errors::Error::NameInUse { name } => Self::NameInUse { name },
            guestpy::errors::Error::Borrow { class, kind } => Self::Borrow { class, kind },
            guestpy::errors::Error::Unsupported { message } => Self::Unsupported { message },
            guestpy::errors::Error::Host(error) => Self::Host(error),
            guestpy::errors::Error::Timeout => Self::Timeout,
            guestpy::errors::Error::Cancelled => Self::Cancelled,
            guestpy::errors::Error::Interrupted => Self::Interrupted,
            guestpy::errors::Error::Closed => Self::Closed,
            guestpy::errors::Error::StopIteration => Self::StopIteration,
            guestpy::errors::Error::StopAsyncIteration => Self::StopAsyncIteration,
            guestpy::errors::Error::Io(error) => Self::Io(error),
            guestpy::errors::Error::Unexpected { message, source } => Self::Unexpected { message, source },
        }
    }
}

/// Errors produced while constructing, operating, or shutting down an executor.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An operation failed inside GuestPy.
    #[error(transparent)]
    Guest(#[from] GuestError),

    /// No Tokio runtime handle is available to capture as the host runtime.
    #[error("no active runtime to capture as the host runtime")]
    NoHostRuntime,

    /// The executor was configured with zero workers.
    #[error("executor worker count must be greater than zero")]
    InvalidWorkerCount,

    /// The executor was configured with zero queue capacity.
    #[error("executor queue capacity must be greater than zero")]
    InvalidQueueCapacity,

    /// A worker's dedicated OS thread failed to spawn.
    #[error("failed to spawn worker {worker}: {source}")]
    WorkerSpawn {
        worker: usize,
        #[source]
        source: io::Error,
    },

    /// A worker's current-thread Tokio runtime failed to build.
    #[error("failed to build runtime for worker {worker}: {source}")]
    WorkerRuntime {
        worker: usize,
        #[source]
        source: io::Error,
    },

    #[error("runtime configuration failed: {0}")]
    Configuration(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// A worker failed to build its GuestPy environment or import the entry module.
    #[error("failed to initialize worker {worker}: {source}")]
    WorkerInitialization {
        worker: usize,
        #[source]
        source: Box<Error>,
    },

    /// A worker's job queue is no longer accepting work.
    #[error("worker {worker} is unavailable")]
    WorkerUnavailable { worker: usize },

    /// A worker dropped its execution response before sending a result.
    #[error("worker {worker} dropped its execution response")]
    WorkerResponseDropped { worker: usize },

    /// A worker failed to shut down its GuestPy environment cleanly.
    #[error("failed to shut down worker {worker}: {source}")]
    WorkerShutdown {
        worker: usize,
        #[source]
        source: GuestError,
    },

    /// The executor has already been shut down.
    #[error("executor is shut down")]
    ExecutorShutdown,

    /// Shutdown was requested from inside one of the executor's own workers.
    #[error("executor cannot shut down from one of its own workers")]
    ReentrantShutdown,

    /// A worker's OS thread panicked.
    #[error("worker {worker} panicked")]
    WorkerPanicked { worker: usize },

    /// The host runtime stopped before an offloaded future could complete.
    #[error("the host runtime did not return a result for the offloaded future")]
    HostRuntimeStopped,

    /// A worker join task failed.
    #[error("worker join task failed: {0}")]
    JoinTask(#[from] JoinError),

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
    pub fn worker_spawn(worker: impl Into<usize>, source: impl Into<io::Error>) -> Self {
        Self::WorkerSpawn {
            worker: worker.into(),
            source: source.into(),
        }
    }

    /// Creates an [`Error::WorkerRuntime`] error.
    pub fn worker_runtime(worker: impl Into<usize>, source: impl Into<io::Error>) -> Self {
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

    /// Creates an [`Error::WorkerShutdown`] error.
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
    pub fn join_task(error: impl Into<JoinError>) -> Self {
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

impl From<guestpy::errors::Error> for Error {
    fn from(error: guestpy::errors::Error) -> Self {
        Self::Guest(GuestError::from(error))
    }
}
