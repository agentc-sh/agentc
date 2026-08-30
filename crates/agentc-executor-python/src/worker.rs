// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    any::Any,
    cell::RefCell,
    rc::{Rc, Weak},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread::JoinHandle,
};

use guestpy::{
    bundle::Bundle,
    runtime::{Runtime, RuntimeBuilder},
};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::{backend::ExecutorBackend, context::Context, errors::Error, job::Job};

static NEXT_EXECUTOR_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static EXECUTION_CONTEXT: RefCell<Option<ExecutionContext>> = const {
        RefCell::new(None)
    };
}

pub(crate) type RuntimeConfiguration<B> =
    Arc<dyn Fn(RuntimeBuilder<B>) -> RuntimeBuilder<B> + Send + Sync>;

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
    context: Weak<dyn Any>,
}

impl ExecutionContext {
    pub(crate) fn install<B>(executor: ExecutorId, worker: WorkerId, context: &Rc<Context<B>>)
    where
        B: ExecutorBackend,
    {
        let erased: Rc<dyn Any> = context.clone();

        EXECUTION_CONTEXT.with(|current| {
            *current.borrow_mut() = Some(Self {
                executor,
                worker,
                context: Rc::downgrade(&erased),
            });
        });
    }

    pub(crate) fn clear() {
        EXECUTION_CONTEXT.with(|current| {
            drop(current.borrow_mut().take());
        });
    }

    pub(crate) fn for_executor<B>(executor: ExecutorId) -> Option<Rc<Context<B>>>
    where
        B: ExecutorBackend,
    {
        EXECUTION_CONTEXT
            .with(|current| {
                current
                    .borrow()
                    .as_ref()
                    .filter(|current| current.executor == executor)
                    .map(|current| current.context.clone())
            })
            .and_then(|context| context.upgrade())
            .and_then(|context| context.downcast::<Context<B>>().ok())
    }

    pub(crate) fn for_worker<B>(executor: ExecutorId, worker: WorkerId) -> Option<Rc<Context<B>>>
    where
        B: ExecutorBackend,
    {
        EXECUTION_CONTEXT
            .with(|current| {
                current
                    .borrow()
                    .as_ref()
                    .filter(|current| current.executor == executor && current.worker == worker)
                    .map(|current| current.context.clone())
            })
            .and_then(|context| context.upgrade())
            .and_then(|context| context.downcast::<Context<B>>().ok())
    }
}

pub(crate) struct WorkerHandle<B: ExecutorBackend> {
    id: WorkerId,
    sender: mpsc::Sender<Box<dyn Job<B>>>,
}

impl<B: ExecutorBackend> WorkerHandle<B> {
    pub(crate) fn spawn(
        config: WorkerConfig<B>,
        queue_capacity: usize,
    ) -> Result<(Self, oneshot::Receiver<Result<(), Error>>, JoinHandle<Result<(), Error>>), Error>
    {
        let worker = config.worker;
        let (sender, receiver) = mpsc::channel(queue_capacity);
        let (startup, ready) = oneshot::channel();
        let thread = std::thread::Builder::new()
            .name(format!("agentc-python-{}-{}", config.executor.value(), worker.index(),))
            .spawn(move || Worker::run(config, receiver, startup))
            .map_err(|error| Error::worker_spawn(worker, error))?;

        Ok((Self { id: worker, sender }, ready, thread))
    }

    pub(crate) fn id(&self) -> WorkerId {
        self.id
    }

    pub(crate) fn sender(&self) -> &mpsc::Sender<Box<dyn Job<B>>> {
        &self.sender
    }
}

#[derive(Clone)]
pub(crate) struct WorkerConfig<B: ExecutorBackend> {
    entry: Arc<str>,
    bundles: Arc<Vec<Bundle>>,
    configurations: Arc<Vec<RuntimeConfiguration<B>>>,
    executor: ExecutorId,
    worker: WorkerId,
    cancellation: CancellationToken,
}

impl<B: ExecutorBackend> WorkerConfig<B> {
    pub(crate) fn builder(entry: Arc<str>, bundles: Arc<Vec<Bundle>>) -> WorkerConfigBuilder<B> {
        WorkerConfigBuilder::new(entry, bundles)
    }
}

pub(crate) struct WorkerConfigBuilder<B: ExecutorBackend> {
    entry: Arc<str>,
    bundles: Arc<Vec<Bundle>>,
    configurations: Option<Arc<Vec<RuntimeConfiguration<B>>>>,
    executor: Option<ExecutorId>,
    worker: Option<WorkerId>,
    cancellation: Option<CancellationToken>,
}

impl<B: ExecutorBackend> WorkerConfigBuilder<B> {
    pub(crate) fn new(entry: Arc<str>, bundles: Arc<Vec<Bundle>>) -> Self {
        Self {
            entry,
            bundles,
            configurations: None,
            executor: None,
            worker: None,
            cancellation: None,
        }
    }

    pub(crate) fn configurations(
        mut self,
        configurations: Arc<Vec<RuntimeConfiguration<B>>>,
    ) -> Self {
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

    pub(crate) fn build(self) -> WorkerConfig<B> {
        WorkerConfig {
            entry: self.entry,
            bundles: self.bundles,
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
    fn run<B: ExecutorBackend>(
        config: WorkerConfig<B>,
        receiver: mpsc::Receiver<Box<dyn Job<B>>>,
        startup: oneshot::Sender<Result<(), Error>>,
    ) -> Result<(), Error> {
        let worker = config.worker;
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = startup.send(Err(Error::worker_runtime(worker, error)));
                return Ok(());
            }
        };
        let local = tokio::task::LocalSet::new();
        let context = runtime.block_on(local.run_until(Self::serve(config, receiver, startup)));

        drop(local);

        let Some(context) = context else {
            return Ok(());
        };
        let context = Rc::try_unwrap(context).map_err(|_| {
            Error::worker_shutdown(
                worker,
                guestpy::errors::Error::unexpected(
                    "worker context remained referenced during shutdown",
                ),
            )
        })?;

        runtime
            .block_on(tokio::task::LocalSet::new().run_until(context.shutdown()))
            .map_err(|error| Error::worker_shutdown(worker, error))
    }

    async fn initialize<B: ExecutorBackend>(
        config: &WorkerConfig<B>,
    ) -> Result<Rc<Context<B>>, guestpy::errors::Error> {
        let runtime = config
            .configurations
            .iter()
            .fold(Runtime::<B>::builder(), |builder, configure| configure(builder));
        let runtime = config
            .bundles
            .iter()
            .cloned()
            .fold(runtime, RuntimeBuilder::bundle)
            .cancellation(config.cancellation.clone())
            .build()?;
        let guest = match runtime.guest().build() {
            Ok(guest) => guest,
            Err(error) => {
                let _ = runtime.shutdown();
                return Err(error);
            }
        };
        let module = match guest.import(config.entry.as_ref()) {
            Ok(module) => module,
            Err(error) => {
                let _ = Context::close_initialization(runtime, guest).await;
                return Err(error);
            }
        };

        Ok(Rc::new(Context::new(runtime, guest, module)))
    }

    async fn serve<B: ExecutorBackend>(
        config: WorkerConfig<B>,
        mut receiver: mpsc::Receiver<Box<dyn Job<B>>>,
        startup: oneshot::Sender<Result<(), Error>>,
    ) -> Option<Rc<Context<B>>> {
        let context = match Self::initialize(&config).await {
            Ok(context) => context,
            Err(error) => {
                let _ = startup.send(Err(Error::worker_initialization(config.worker, error)));
                return None;
            }
        };

        ExecutionContext::install(config.executor, config.worker, &context);

        if startup.send(Ok(())).is_err() {
            ExecutionContext::clear();
            return Some(context);
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

        Some(context)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use guestpy::{bundle::Bundle, runtime::Runtime, rustpython::RustPython};
    use tokio_util::sync::CancellationToken;

    use crate::{
        context::Context,
        worker::{ExecutionContext, ExecutorId, WorkerId},
    };

    fn build_context() -> Rc<Context<RustPython>> {
        let runtime = Runtime::<RustPython>::builder()
            .bundle(Bundle::single("test_component", "value = 42\n").unwrap())
            .cancellation(CancellationToken::new())
            .build()
            .unwrap();
        let guest = runtime.guest().build().unwrap();
        let module = guest.import("test_component").unwrap();

        Rc::new(Context::new(runtime, guest, module))
    }

    #[test]
    fn executor_ids_never_use_zero() {
        let first = ExecutorId::next();
        let second = ExecutorId::next();

        assert_ne!(first, second);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn execution_context_matches_executor_and_worker() {
        let context = build_context();
        let executor = ExecutorId::next();
        let worker = WorkerId::new(0);
        let other_executor = ExecutorId::next();
        let other_worker = WorkerId::new(1);

        ExecutionContext::install(executor, worker, &context);

        assert!(ExecutionContext::for_executor::<RustPython>(executor).is_some());
        assert!(ExecutionContext::for_worker::<RustPython>(executor, worker).is_some());
        assert!(ExecutionContext::for_executor::<RustPython>(other_executor).is_none());
        assert!(ExecutionContext::for_worker::<RustPython>(executor, other_worker).is_none());

        ExecutionContext::clear();

        let Ok(context) = Rc::try_unwrap(context) else {
            panic!("test retained the worker context");
        };

        context.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn execution_context_does_not_extend_context_lifetime() {
        let context = build_context();
        let executor = ExecutorId::next();
        let worker = WorkerId::new(0);

        ExecutionContext::install(executor, worker, &context);

        let Ok(context) = Rc::try_unwrap(context) else {
            panic!("test retained the worker context");
        };

        context.shutdown().await.unwrap();

        assert!(ExecutionContext::for_executor::<RustPython>(executor).is_none());

        ExecutionContext::clear();
    }
}
