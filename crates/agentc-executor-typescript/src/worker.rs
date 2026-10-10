// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread::JoinHandle,
};

use guestjs::runtime::{Runtime, RuntimeBuilder};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::{context::Context, error::Error, job::Job};

static NEXT_EXECUTOR_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static EXECUTION_CONTEXT: RefCell<Option<ExecutionContext>> = const {
        RefCell::new(None)
    };
}

pub(crate) type RuntimeConfiguration = Arc<dyn Fn(RuntimeBuilder) -> RuntimeBuilder + Send + Sync>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ExecutorId(u64);

impl ExecutorId {
    pub(crate) fn next() -> Self {
        loop {
            let id = NEXT_EXECUTOR_ID.fetch_add(1, Ordering::Relaxed);

            if id != 0 {
                return Self(id);
            }
        }
    }

    fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct WorkerId(usize);

impl WorkerId {
    pub(crate) fn new(index: usize) -> Self {
        Self(index)
    }

    pub(crate) fn index(self) -> usize {
        self.0
    }
}

impl From<WorkerId> for usize {
    fn from(worker: WorkerId) -> Self {
        worker.index()
    }
}

pub(crate) struct ExecutionContext {
    executor: ExecutorId,
    worker: WorkerId,
    context: Weak<Context>,
}

impl ExecutionContext {
    pub(crate) fn install(executor: ExecutorId, worker: WorkerId, context: &Rc<Context>) {
        EXECUTION_CONTEXT.with(|current| {
            *current.borrow_mut() = Some(Self {
                executor,
                worker,
                context: Rc::downgrade(context),
            });
        });
    }

    pub(crate) fn clear() {
        EXECUTION_CONTEXT.with(|current| {
            drop(current.borrow_mut().take());
        });
    }

    pub(crate) fn for_executor(executor: ExecutorId) -> Option<Rc<Context>> {
        EXECUTION_CONTEXT
            .with(|current| {
                current
                    .borrow()
                    .as_ref()
                    .filter(|current| current.executor == executor)
                    .map(|current| current.context.clone())
            })
            .and_then(|context| context.upgrade())
    }

    pub(crate) fn for_worker(executor: ExecutorId, worker: WorkerId) -> Option<Rc<Context>> {
        EXECUTION_CONTEXT
            .with(|current| {
                current
                    .borrow()
                    .as_ref()
                    .filter(|current| current.executor == executor && current.worker == worker)
                    .map(|current| current.context.clone())
            })
            .and_then(|context| context.upgrade())
    }
}

pub(crate) struct WorkerHandle {
    id: WorkerId,
    sender: mpsc::Sender<Box<dyn Job>>,
}

impl WorkerHandle {
    pub(crate) fn spawn(
        config: WorkerConfig,
        queue_capacity: usize,
    ) -> Result<(Self, oneshot::Receiver<Result<(), Error>>, JoinHandle<()>), Error> {
        let worker = config.worker;
        let (sender, receiver) = mpsc::channel(queue_capacity);
        let (startup, ready) = oneshot::channel();

        let thread = std::thread::Builder::new()
            .name(format!("agentc-typescript-{}-{}", config.executor.value(), worker.index(),))
            .spawn(move || Worker::run(config, receiver, startup))
            .map_err(|error| Error::worker_spawn(worker, error))?;

        Ok((Self { id: worker, sender }, ready, thread))
    }

    pub(crate) fn id(&self) -> WorkerId {
        self.id
    }

    pub(crate) fn sender(&self) -> &mpsc::Sender<Box<dyn Job>> {
        &self.sender
    }
}

#[derive(Clone)]
pub(crate) struct WorkerConfig {
    name: Arc<str>,
    source: Arc<str>,
    configurations: Arc<Vec<RuntimeConfiguration>>,
    executor: ExecutorId,
    worker: WorkerId,
    cancellation: CancellationToken,
}

impl WorkerConfig {
    pub(crate) fn builder(name: Arc<str>, source: Arc<str>) -> WorkerConfigBuilder {
        WorkerConfigBuilder::new(name, source)
    }
}

pub(crate) struct WorkerConfigBuilder {
    name: Arc<str>,
    source: Arc<str>,
    configurations: Option<Arc<Vec<RuntimeConfiguration>>>,
    executor: Option<ExecutorId>,
    worker: Option<WorkerId>,
    cancellation: Option<CancellationToken>,
}

impl WorkerConfigBuilder {
    pub(crate) fn new(name: Arc<str>, source: Arc<str>) -> Self {
        Self {
            name,
            source,
            configurations: None,
            executor: None,
            worker: None,
            cancellation: None,
        }
    }

    pub(crate) fn configurations(mut self, configurations: Arc<Vec<RuntimeConfiguration>>) -> Self {
        self.configurations = Some(configurations);
        self
    }

    pub(crate) fn executor(mut self, executor: ExecutorId) -> Self {
        self.executor = Some(executor);
        self
    }

    pub(crate) fn worker(mut self, worker: WorkerId) -> Self {
        self.worker = Some(worker);
        self
    }

    pub(crate) fn cancellation(mut self, cancellation: CancellationToken) -> Self {
        self.cancellation = Some(cancellation);
        self
    }

    pub(crate) fn build(self) -> WorkerConfig {
        WorkerConfig {
            name: self.name,
            source: self.source,
            configurations: self
                .configurations
                .expect("WorkerConfigBuilder: configurations not set"),
            executor: self
                .executor
                .expect("WorkerConfigBuilder: executor not set"),
            worker: self
                .worker
                .expect("WorkerConfigBuilder: worker not set"),
            cancellation: self
                .cancellation
                .expect("WorkerConfigBuilder: cancellation not set"),
        }
    }
}

struct Worker;

impl Worker {
    fn run(
        config: WorkerConfig,
        receiver: mpsc::Receiver<Box<dyn Job>>,
        startup: oneshot::Sender<Result<(), Error>>,
    ) {
        match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime.block_on(
                tokio::task::LocalSet::new().run_until(Self::serve(config, receiver, startup)),
            ),
            Err(error) => {
                let _ = startup.send(Err(Error::worker_runtime(config.worker, error)));
            }
        }
    }

    async fn serve(
        config: WorkerConfig,
        mut receiver: mpsc::Receiver<Box<dyn Job>>,
        startup: oneshot::Sender<Result<(), Error>>,
    ) {
        let context = match Self::initialize(&config).await {
            Ok(context) => context,
            Err(error) => {
                let _ = startup.send(Err(Error::worker_initialization(config.worker, error)));
                return;
            }
        };

        ExecutionContext::install(config.executor, config.worker, &context);

        if startup.send(Ok(())).is_err() {
            ExecutionContext::clear();
            return;
        }

        loop {
            tokio::select! {
                _ = config.cancellation.cancelled() => break,
                job = receiver.recv() => {
                    let Some(job) = job else {
                        break;
                    };

                    tokio::select! {
                        _ = config.cancellation.cancelled() => break,
                        _ = job.execute(context.clone()) => {}
                    }
                }
            }
        }

        ExecutionContext::clear();
    }

    async fn initialize(config: &WorkerConfig) -> Result<Rc<Context>, guestjs::errors::Error> {
        let runtime = config
            .configurations
            .iter()
            .fold(Runtime::builder(), |builder, configure| configure(builder))
            .cancellation(config.cancellation.clone())
            .build()
            .await?;
        let guest = runtime.guest().build().await?;
        let module = guest
            .guest_module(config.name.as_ref(), config.source.as_ref())
            .await?;

        Ok(Rc::new(Context::new(runtime, guest, module)))
    }
}
