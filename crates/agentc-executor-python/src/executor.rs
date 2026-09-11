// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread::JoinHandle,
};

use futures::future::LocalBoxFuture;
use guestpy::{bundle::Bundle, runtime::RuntimeBuilder};
use tokio_util::sync::CancellationToken;

use crate::{
    backend::ExecutorBackend,
    context::Context,
    errors::Error,
    execution::Execution,
    job::TypedJob,
    lease::WorkerLease,
    worker::{
        ExecutionContext, ExecutorId, RuntimeConfiguration, WorkerConfig, WorkerHandle, WorkerId,
    },
};

pub(crate) struct ExecutorInner<B: ExecutorBackend> {
    id: ExecutorId,
    workers: Vec<WorkerHandle<B>>,
    next_worker: AtomicUsize,
    shutdown: CancellationToken,
    joins: Mutex<Option<Vec<(WorkerId, JoinHandle<Result<(), Error>>)>>>,
}

impl<B: ExecutorBackend> ExecutorInner<B> {
    pub(crate) fn id(&self) -> ExecutorId {
        self.id
    }

    pub(crate) fn is_shutdown(&self) -> bool {
        self.shutdown.is_cancelled()
    }

    pub(crate) fn next_worker(&self) -> WorkerId {
        WorkerId::new(
            self.next_worker
                .fetch_add(1, Ordering::Relaxed)
                % self.workers.len(),
        )
    }

    pub(crate) fn dispatch<F, T>(&self, selected: WorkerId, operation: F) -> Execution<T>
    where
        F: for<'a> FnOnce(&'a Context<B>) -> LocalBoxFuture<'a, Result<T, guestpy::errors::Error>>
            + Send
            + 'static,
        T: Send + 'static,
    {
        if self.shutdown.is_cancelled() {
            return Execution::ready(Err(Error::executor_shutdown()));
        }

        let (job, response) = TypedJob::prepare(operation);
        let worker = &self.workers[selected.index()];
        let id = worker.id();
        let sender = worker.sender().clone();

        Execution::new(async move {
            sender
                .send(job)
                .await
                .map_err(|_| Error::worker_unavailable(id))?;

            response
                .await
                .map_err(|_| Error::worker_response_dropped(id))?
        })
    }

    pub(crate) fn dispatch_local<F, T>(&self, context: Rc<Context<B>>, operation: F) -> Execution<T>
    where
        F: for<'a> FnOnce(&'a Context<B>) -> LocalBoxFuture<'a, Result<T, guestpy::errors::Error>>
            + Send
            + 'static,
        T: Send + 'static,
    {
        if self.shutdown.is_cancelled() {
            return Execution::ready(Err(Error::executor_shutdown()));
        }

        let (job, response) = TypedJob::prepare(operation);

        drop(tokio::task::spawn_local(job.execute(context)));

        Execution::new(async move {
            response
                .await
                .map_err(|_| Error::executor_shutdown())?
        })
    }

    pub(crate) async fn join_workers(
        joins: Vec<(WorkerId, JoinHandle<Result<(), Error>>)>,
    ) -> Result<(), Error> {
        tokio::task::spawn_blocking(move || {
            let mut failure = None;

            for (worker, join) in joins {
                match join.join() {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) if failure.is_none() => failure = Some(error),
                    Ok(Err(_)) => {}
                    Err(_) if failure.is_none() => {
                        failure = Some(Error::worker_panicked(worker));
                    }
                    Err(_) => {}
                }
            }

            match failure {
                Some(error) => Err(error),
                None => Ok(()),
            }
        })
        .await
        .map_err(Error::join_task)?
    }
}

impl<B: ExecutorBackend> Drop for ExecutorInner<B> {
    fn drop(&mut self) {
        self.shutdown.cancel();
    }
}

/// A cloneable handle to a persistent Python package executor.
pub struct Executor<B: ExecutorBackend> {
    inner: Arc<ExecutorInner<B>>,
}

impl<B: ExecutorBackend> Clone for Executor<B> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<B: ExecutorBackend> Executor<B> {
    /// Creates a builder for a Python entry module.
    pub fn builder(entry: impl Into<String>) -> ExecutorBuilder<B> {
        ExecutorBuilder::new(entry)
    }

    /// Executes a typed GuestPy operation on a worker.
    pub fn execute<F, T>(&self, operation: F) -> Execution<T>
    where
        F: for<'a> FnOnce(&'a Context<B>) -> LocalBoxFuture<'a, Result<T, guestpy::errors::Error>>
            + Send
            + 'static,
        T: Send + 'static,
    {
        if let Some(context) = ExecutionContext::for_executor::<B>(self.inner.id()) {
            return self
                .inner
                .dispatch_local(context, operation);
        }

        self.inner
            .dispatch(self.inner.next_worker(), operation)
    }

    /// Creates a stable handle to one worker selected by round robin.
    pub fn lease(&self) -> WorkerLease<B> {
        WorkerLease::new(self.inner.clone(), self.inner.next_worker())
    }

    /// Returns the number of persistent workers owned by the executor.
    pub fn worker_count(&self) -> usize {
        self.inner.workers.len()
    }

    /// Returns whether executor shutdown has begun.
    pub fn is_shutdown(&self) -> bool {
        self.inner.is_shutdown()
    }

    /// Cancels active work and joins every worker thread.
    pub async fn shutdown(&self) -> Result<(), Error> {
        if ExecutionContext::for_executor::<B>(self.inner.id()).is_some() {
            return Err(Error::reentrant_shutdown());
        }

        self.inner.shutdown.cancel();

        let Some(joins) = self
            .inner
            .joins
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
        else {
            return Ok(());
        };

        ExecutorInner::<B>::join_workers(joins).await
    }
}

/// Configures a persistent Python package executor.
pub struct ExecutorBuilder<B: ExecutorBackend> {
    entry: String,
    bundles: Vec<Bundle>,
    workers: usize,
    queue_capacity: usize,
    configurations: Vec<RuntimeConfiguration<B>>,
    cancellation: Option<CancellationToken>,
}

impl<B: ExecutorBackend> ExecutorBuilder<B> {
    const DEFAULT_QUEUE_CAPACITY: usize = 64;

    /// Creates a builder for a Python entry module.
    pub fn new(entry: impl Into<String>) -> Self {
        Self {
            entry: entry.into(),
            bundles: Vec::new(),
            workers: std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1),
            queue_capacity: Self::DEFAULT_QUEUE_CAPACITY,
            configurations: Vec::new(),
            cancellation: None,
        }
    }

    async fn cleanup_startup(
        error: Error,
        shutdown: &CancellationToken,
        joins: Vec<(WorkerId, JoinHandle<Result<(), Error>>)>,
    ) -> Error {
        shutdown.cancel();
        let _ = ExecutorInner::<B>::join_workers(joins).await;

        error
    }

    /// Adds one GuestPy package bundle.
    pub fn bundle(mut self, bundle: Bundle) -> Self {
        self.bundles.push(bundle);
        self
    }

    /// Adds several GuestPy package bundles.
    pub fn bundles<I>(mut self, bundles: I) -> Self
    where
        I: IntoIterator<Item = Bundle>,
    {
        self.bundles.extend(bundles);
        self
    }

    /// Sets the number of persistent package workers.
    pub fn workers(mut self, workers: usize) -> Self {
        self.workers = workers;
        self
    }

    /// Sets the pending job capacity of each worker queue.
    pub fn queue_capacity(mut self, queue_capacity: usize) -> Self {
        self.queue_capacity = queue_capacity;
        self
    }

    /// Adds a GuestPy runtime configuration applied locally to every worker.
    pub fn configure<F>(mut self, configure: F) -> Self
    where
        F: Fn(RuntimeBuilder<B>) -> RuntimeBuilder<B> + Send + Sync + 'static,
    {
        self.configurations
            .push(Arc::new(configure));
        self
    }

    /// Sets the shared token used for executor and guest cancellation.
    pub fn cancellation(mut self, cancellation: CancellationToken) -> Self {
        self.cancellation = Some(cancellation);
        self
    }

    /// Builds and initializes every persistent package worker.
    pub async fn build(self) -> Result<Executor<B>, Error> {
        if self.workers == 0 {
            return Err(Error::invalid_worker_count());
        }

        if self.queue_capacity == 0 {
            return Err(Error::invalid_queue_capacity());
        }

        let executor = ExecutorId::next();
        let shutdown = self.cancellation.unwrap_or_default();
        let entry = Arc::<str>::from(self.entry);
        let bundles = Arc::new(self.bundles);
        let configurations = Arc::new(self.configurations);
        let mut workers = Vec::with_capacity(self.workers);
        let mut startups = Vec::with_capacity(self.workers);
        let mut joins = Vec::with_capacity(self.workers);

        for index in 0..self.workers {
            let worker = WorkerId::new(index);

            match WorkerHandle::spawn(
                WorkerConfig::builder(entry.clone(), bundles.clone())
                    .configurations(configurations.clone())
                    .executor(executor)
                    .worker(worker)
                    .cancellation(shutdown.clone())
                    .build(),
                self.queue_capacity,
            ) {
                Ok((handle, startup, join)) => {
                    workers.push(handle);
                    startups.push((worker, startup));
                    joins.push((worker, join));
                }
                Err(error) => {
                    return Err(Self::cleanup_startup(error, &shutdown, joins).await);
                }
            }
        }

        for (worker, startup) in startups {
            match startup.await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    return Err(Self::cleanup_startup(error, &shutdown, joins).await);
                }
                Err(_) => {
                    return Err(Self::cleanup_startup(
                        Error::worker_panicked(worker),
                        &shutdown,
                        joins,
                    )
                    .await);
                }
            }
        }

        Ok(Executor {
            inner: Arc::new(ExecutorInner {
                id: executor,
                workers,
                next_worker: AtomicUsize::new(0),
                shutdown,
                joins: Mutex::new(Some(joins)),
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use guestpy::{
        bundle::Bundle,
        handle::{Coroutine, ObjectProtocol},
        pyo3::CPython,
        rustpython::RustPython,
    };
    use tokio::sync::Barrier;
    use tokio_util::sync::CancellationToken;

    use crate::{
        backend::ExecutorBackend,
        errors::{Error, GuestError},
        execution::Execution,
        executor::Executor,
    };

    const COMPONENT_SOURCE: &str = "\
count = 0

def increment():
    global count
    count += 1
    return count

async def increment_async():
    return increment()

def fail():
    raise RuntimeError(\"nested failure\")
";

    struct TestExecutor;

    impl TestExecutor {
        async fn build(workers: usize) -> Executor<RustPython> {
            Executor::<RustPython>::builder("test_component")
                .bundle(Bundle::single("test_component", COMPONENT_SOURCE).unwrap())
                .workers(workers)
                .build()
                .await
                .unwrap()
        }

        fn increment(executor: &Executor<RustPython>) -> Execution<i64> {
            executor.execute(|context| {
                Box::pin(async move {
                    context
                        .module()
                        .function("increment")?
                        .call::<_, i64>(())
                })
            })
        }

        fn increment_recursively(
            executor: &Executor<RustPython>,
            depth: usize,
        ) -> Execution<Vec<i64>> {
            let nested = executor.clone();

            executor.execute(move |context| {
                Box::pin(async move {
                    let value = context
                        .module()
                        .function("increment")?
                        .call::<_, i64>(())?;

                    if depth == 1 {
                        return Ok(vec![value]);
                    }

                    let mut values = Self::increment_recursively(&nested, depth - 1)
                        .await
                        .map_err(|error| guestpy::errors::Error::unexpected(error.to_string()))?;

                    values.insert(0, value);

                    Ok(values)
                })
            })
        }
    }

    struct BackendContract;

    impl BackendContract {
        fn bundle() -> Bundle {
            Bundle::builder()
                .package("executor_fixture", "")
                .module("executor_fixture.helpers", "def unit():\n    return 1\n")
                .module(
                    "executor_fixture.component",
                    "from executor_fixture.helpers import unit\n\ncount = 0\n\ndef increment():\n    global count\n    count += unit()\n    return count\n",
                )
                .build()
                .unwrap()
        }

        async fn assert_execution<B: ExecutorBackend>() {
            let executor = Executor::<B>::builder("executor_fixture.component")
                .bundle(Self::bundle())
                .workers(1)
                .build()
                .await
                .unwrap();

            assert_eq!(
                executor
                    .execute(|context| Box::pin(async move {
                        context
                            .module()
                            .function("increment")?
                            .call::<_, i64>(())
                    }))
                    .await
                    .unwrap(),
                1,
            );

            executor.shutdown().await.unwrap();
        }

        async fn assert_lease<B: ExecutorBackend>() {
            let executor = Executor::<B>::builder("executor_fixture.component")
                .bundle(Self::bundle())
                .workers(1)
                .build()
                .await
                .unwrap();
            let lease = executor.lease();
            let first = lease
                .execute(|context| {
                    Box::pin(async move {
                        context
                            .module()
                            .function("increment")?
                            .call::<_, i64>(())
                    })
                })
                .await
                .unwrap();
            let second = lease
                .execute(|context| {
                    Box::pin(async move {
                        context
                            .module()
                            .function("increment")?
                            .call::<_, i64>(())
                    })
                })
                .await
                .unwrap();

            assert_eq!((first, second), (1, 2));

            executor.shutdown().await.unwrap();
        }
    }

    #[tokio::test]
    async fn rejects_invalid_worker_configuration() {
        let Err(worker_error) = Executor::<RustPython>::builder("test_component")
            .workers(0)
            .build()
            .await
        else {
            panic!("zero workers should be rejected");
        };
        let Err(queue_error) = Executor::<RustPython>::builder("test_component")
            .queue_capacity(0)
            .build()
            .await
        else {
            panic!("zero queue capacity should be rejected");
        };

        assert!(matches!(worker_error, Error::InvalidWorkerCount));
        assert!(matches!(queue_error, Error::InvalidQueueCapacity));
    }

    #[tokio::test]
    async fn missing_entry_module_fails_complete_build() {
        let Err(error) = Executor::<RustPython>::builder("missing.component")
            .workers(2)
            .build()
            .await
        else {
            panic!("a missing entry module should fail initialization");
        };

        assert!(matches!(error, Error::WorkerInitialization { .. }));
    }

    #[tokio::test]
    async fn configures_every_worker() {
        let configurations = Arc::new(AtomicUsize::new(0));
        let executor = Executor::<RustPython>::builder("test_component")
            .bundle(Bundle::single("test_component", COMPONENT_SOURCE).unwrap())
            .workers(3)
            .configure({
                let configurations = configurations.clone();

                move |builder| {
                    configurations.fetch_add(1, Ordering::SeqCst);
                    builder
                }
            })
            .build()
            .await
            .unwrap();

        assert_eq!(configurations.load(Ordering::SeqCst), 3);

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn rotates_between_persistent_worker_replicas() {
        let executor = TestExecutor::build(2).await;

        assert_eq!(
            [
                TestExecutor::increment(&executor)
                    .await
                    .unwrap(),
                TestExecutor::increment(&executor)
                    .await
                    .unwrap(),
                TestExecutor::increment(&executor)
                    .await
                    .unwrap(),
                TestExecutor::increment(&executor)
                    .await
                    .unwrap(),
            ],
            [1, 1, 2, 2],
        );

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn workers_execute_concurrently() {
        let executor = TestExecutor::build(2).await;
        let barrier = Arc::new(Barrier::new(2));
        let first = executor.execute({
            let barrier = barrier.clone();

            move |_context| {
                Box::pin(async move {
                    barrier.wait().await;
                    Ok(())
                })
            }
        });
        let second = executor.execute(move |_context| {
            Box::pin(async move {
                barrier.wait().await;
                Ok(())
            })
        });

        tokio::time::timeout(Duration::from_secs(1), async move {
            let (first, second) = tokio::join!(first, second);

            first.unwrap();
            second.unwrap();
        })
        .await
        .unwrap();

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn nested_execution_reuses_worker_context() {
        let executor = TestExecutor::build(1).await;
        let nested = executor.clone();

        assert_eq!(
            tokio::time::timeout(
                Duration::from_secs(1),
                executor.execute(move |context| {
                    Box::pin(async move {
                        let first = context
                            .module()
                            .function("increment")?
                            .call::<_, i64>(())?;
                        let second = nested
                            .execute(|context| {
                                Box::pin(async move {
                                    context
                                        .module()
                                        .function("increment")?
                                        .call::<_, i64>(())
                                })
                            })
                            .await
                            .map_err(|error| {
                                guestpy::errors::Error::unexpected(error.to_string())
                            })?;

                        Ok((first, second))
                    })
                }),
            )
            .await
            .unwrap()
            .unwrap(),
            (1, 2),
        );

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn supports_three_levels_of_reentry() {
        let executor = TestExecutor::build(1).await;

        assert_eq!(
            tokio::time::timeout(
                Duration::from_secs(1),
                TestExecutor::increment_recursively(&executor, 3),
            )
            .await
            .unwrap()
            .unwrap(),
            [1, 2, 3],
        );

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn nested_guest_error_preserves_worker() {
        let executor = TestExecutor::build(1).await;
        let nested = executor.clone();
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            executor.execute(move |_context| {
                Box::pin(async move {
                    Ok(nested
                        .execute(|context| {
                            Box::pin(async move {
                                context
                                    .module()
                                    .function("fail")?
                                    .call::<_, ()>(())
                            })
                        })
                        .await)
                })
            }),
        )
        .await
        .unwrap()
        .unwrap();

        assert!(matches!(result, Err(Error::Guest(_))));
        assert_eq!(
            TestExecutor::increment(&executor)
                .await
                .unwrap(),
            1
        );

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn rejects_shutdown_from_own_worker() {
        let executor = TestExecutor::build(1).await;
        let nested = executor.clone();
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            executor.execute(move |_context| Box::pin(async move { Ok(nested.shutdown().await) })),
        )
        .await
        .unwrap()
        .unwrap();

        assert!(matches!(result, Err(Error::ReentrantShutdown)));
        assert!(!executor.is_shutdown());

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn caller_cancellation_shuts_down_executor() {
        let cancellation = CancellationToken::new();
        let executor = Executor::<RustPython>::builder("test_component")
            .bundle(Bundle::single("test_component", COMPONENT_SOURCE).unwrap())
            .workers(1)
            .cancellation(cancellation.clone())
            .build()
            .await
            .unwrap();

        cancellation.cancel();

        assert!(executor.is_shutdown());
        assert!(matches!(
            executor
                .execute(|_context| Box::pin(async move { Ok(()) }))
                .await,
            Err(Error::ExecutorShutdown),
        ));

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn shutdown_rejects_later_work() {
        let executor = TestExecutor::build(1).await;

        executor.shutdown().await.unwrap();
        executor.shutdown().await.unwrap();

        assert!(matches!(
            executor
                .execute(|_context| Box::pin(async move { Ok(()) }))
                .await,
            Err(Error::ExecutorShutdown),
        ));
    }

    #[tokio::test]
    async fn awaits_python_coroutine() {
        let executor = TestExecutor::build(1).await;

        assert_eq!(
            executor
                .execute(|context| {
                    Box::pin(async move {
                        context
                            .module()
                            .function("increment_async")?
                            .call::<_, Coroutine<RustPython, i64>>(())?
                            .await
                    })
                })
                .await
                .unwrap(),
            1,
        );

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn executes_with_rustpython() {
        BackendContract::assert_execution::<RustPython>().await;
    }

    #[tokio::test]
    async fn executes_with_cpython() {
        BackendContract::assert_execution::<CPython>().await;
    }

    #[tokio::test]
    async fn lease_executes_with_rustpython() {
        BackendContract::assert_lease::<RustPython>().await;
    }

    #[tokio::test]
    async fn lease_executes_with_cpython() {
        BackendContract::assert_lease::<CPython>().await;
    }

    #[tokio::test]
    async fn native_extension_is_rejected_only_when_imported() {
        let bundle = Bundle::builder()
            .package("native_fixture", "")
            .data(
                "native_fixture/_native.cpython-313-x86_64-linux-gnu.so",
                b"native-bytes".to_vec(),
            )
            .build()
            .unwrap();
        let executor = Executor::<RustPython>::builder("native_fixture")
            .bundle(bundle)
            .workers(1)
            .build()
            .await
            .unwrap();

        let Err(Error::Guest(GuestError::Unsupported { ref message })) = executor
            .execute(|context| {
                Box::pin(async move {
                    context
                        .guest()
                        .import("native_fixture._native")
                        .map(|_| ())
                })
            })
            .await
        else {
            panic!("importing a native extension under rustpython should fail");
        };

        assert!(message.contains("native_fixture._native"));

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn startup_failure_joins_started_workers() {
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            Executor::<RustPython>::builder("missing.component")
                .workers(4)
                .build(),
        )
        .await
        .unwrap();

        assert!(matches!(result, Err(Error::WorkerInitialization { .. })));
    }

    #[tokio::test]
    async fn shutdown_joins_worker_threads() {
        let executor = TestExecutor::build(2).await;

        executor.shutdown().await.unwrap();
        executor.shutdown().await.unwrap();

        assert!(matches!(
            executor
                .execute(|_context| Box::pin(async move { Ok(()) }))
                .await,
            Err(Error::ExecutorShutdown),
        ));
    }
}
