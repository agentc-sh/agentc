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
use guestjs::runtime::RuntimeBuilder;
use tokio_util::sync::CancellationToken;

use crate::{
    context::Context,
    error::Error,
    execution::Execution,
    job::TypedJob,
    lease::WorkerLease,
    worker::{
        ExecutionContext, ExecutorId, RuntimeConfiguration, WorkerConfig, WorkerHandle, WorkerId,
    },
};

/// A cloneable handle to a persistent TypeScript package executor.
#[derive(Clone)]
pub struct Executor {
    inner: Arc<ExecutorInner>,
}

impl Executor {
    /// Creates a builder for a named TypeScript package module.
    pub fn builder(name: impl Into<String>, source: impl Into<String>) -> ExecutorBuilder {
        ExecutorBuilder::new(name, source)
    }

    /// Executes a typed GuestJS operation on a worker.
    pub fn execute<F, T>(&self, operation: F) -> Execution<T>
    where
        F: for<'a> FnOnce(&'a Context) -> LocalBoxFuture<'a, Result<T, guestjs::errors::Error>>
            + Send
            + 'static,
        T: Send + 'static,
    {
        if let Some(context) = ExecutionContext::for_executor(self.inner.id) {
            return self
                .inner
                .dispatch_local(context, operation);
        }

        self.inner
            .dispatch(self.inner.next_worker(), operation)
    }

    /// Creates a stable handle to one worker selected by round robin.
    pub fn lease(&self) -> WorkerLease {
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
        if ExecutionContext::for_executor(self.inner.id).is_some() {
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

        ExecutorInner::join_workers(joins).await
    }
}

/// Configures a persistent TypeScript package executor.
pub struct ExecutorBuilder {
    name: String,
    source: String,
    workers: usize,
    queue_capacity: usize,
    configurations: Vec<RuntimeConfiguration>,
    cancellation: Option<CancellationToken>,
}

impl ExecutorBuilder {
    const DEFAULT_QUEUE_CAPACITY: usize = 64;

    /// Creates a builder for a named TypeScript package module.
    pub fn new(name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
            workers: std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1),
            queue_capacity: Self::DEFAULT_QUEUE_CAPACITY,
            configurations: Vec::new(),
            cancellation: None,
        }
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

    /// Adds a GuestJS runtime configuration applied locally to every worker.
    pub fn configure<F>(mut self, configure: F) -> Self
    where
        F: Fn(RuntimeBuilder) -> RuntimeBuilder + Send + Sync + 'static,
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
    pub async fn build(self) -> Result<Executor, Error> {
        if self.workers == 0 {
            return Err(Error::invalid_worker_count());
        }

        if self.queue_capacity == 0 {
            return Err(Error::invalid_queue_capacity());
        }

        let executor = ExecutorId::next();
        let shutdown = self.cancellation.unwrap_or_default();
        let name = Arc::<str>::from(self.name);
        let source = Arc::<str>::from(self.source);
        let configurations = Arc::new(self.configurations);
        let mut workers = Vec::with_capacity(self.workers);
        let mut startups = Vec::with_capacity(self.workers);
        let mut joins = Vec::with_capacity(self.workers);

        for index in 0..self.workers {
            let worker = WorkerId::new(index);

            match WorkerHandle::spawn(
                WorkerConfig::builder(name.clone(), source.clone())
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

    async fn cleanup_startup(
        error: Error,
        shutdown: &CancellationToken,
        joins: Vec<(WorkerId, JoinHandle<()>)>,
    ) -> Error {
        shutdown.cancel();

        match ExecutorInner::join_workers(joins).await {
            Ok(()) => error,
            Err(error) => error,
        }
    }
}

pub(crate) struct ExecutorInner {
    pub(crate) id: ExecutorId,
    workers: Vec<WorkerHandle>,
    next_worker: AtomicUsize,
    shutdown: CancellationToken,
    joins: Mutex<Option<Vec<(WorkerId, JoinHandle<()>)>>>,
}

impl ExecutorInner {
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
        F: for<'a> FnOnce(&'a Context) -> LocalBoxFuture<'a, Result<T, guestjs::errors::Error>>
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

    pub(crate) fn dispatch_local<F, T>(&self, context: Rc<Context>, operation: F) -> Execution<T>
    where
        F: for<'a> FnOnce(&'a Context) -> LocalBoxFuture<'a, Result<T, guestjs::errors::Error>>
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

    async fn join_workers(joins: Vec<(WorkerId, JoinHandle<()>)>) -> Result<(), Error> {
        tokio::task::spawn_blocking(move || {
            let mut failure = None;

            for (worker, join) in joins {
                if join.join().is_err() && failure.is_none() {
                    failure = Some(Error::worker_panicked(worker));
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

impl Drop for ExecutorInner {
    fn drop(&mut self) {
        self.shutdown.cancel();
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

    use guestjs::handle::Promise;
    use tokio::sync::{Barrier, Notify};
    use tokio_util::sync::CancellationToken;

    use crate::{error::Error, execution::Execution, executor::Executor};

    const COUNTER_SOURCE: &str = r#"
let count = 0;

export function increment() {
    count += 1;
    return count;
}

export async function incrementAsync() {
    await Promise.resolve();
    return increment();
}

export function fail() {
    throw new Error("nested failure");
}
"#;

    struct TestExecutor;

    impl TestExecutor {
        async fn build(workers: usize, source: &str) -> Executor {
            Executor::builder("test.ts", source)
                .workers(workers)
                .build()
                .await
                .unwrap()
        }

        fn increment(executor: &Executor) -> Execution<i32> {
            executor.execute(|context| {
                Box::pin(async move {
                    context
                        .module()
                        .function("increment")
                        .await?
                        .call::<_, i32>(())
                        .await
                })
            })
        }

        fn increment_recursively(executor: &Executor, depth: usize) -> Execution<Vec<i32>> {
            let nested = executor.clone();

            executor.execute(move |context| {
                Box::pin(async move {
                    let value = context
                        .module()
                        .function("increment")
                        .await?
                        .call::<_, i32>(())
                        .await?;

                    if depth == 1 {
                        return Ok(vec![value]);
                    }

                    let mut values = Self::increment_recursively(&nested, depth - 1)
                        .await
                        .map_err(|error| guestjs::errors::Error::unexpected(error.to_string()))?;

                    values.insert(0, value);

                    Ok(values)
                })
            })
        }
    }

    #[tokio::test]
    async fn rejects_invalid_worker_configuration() {
        let Err(worker_error) = Executor::builder("test.ts", "export {}")
            .workers(0)
            .build()
            .await
        else {
            panic!("zero workers should be rejected");
        };
        let Err(queue_error) = Executor::builder("test.ts", "export {}")
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
    async fn invalid_typescript_fails_complete_build() {
        let Err(error) = Executor::builder("invalid.ts", "export function broken(: {")
            .workers(2)
            .build()
            .await
        else {
            panic!("invalid TypeScript should fail initialization");
        };

        assert!(matches!(
            error,
            Error::WorkerInitialization {
                source: guestjs::errors::Error::Transpile { .. },
                ..
            },
        ));
    }

    #[tokio::test]
    async fn configures_every_worker() {
        let configurations = Arc::new(AtomicUsize::new(0));
        let executor = Executor::builder("test.ts", "export {}")
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
        let executor = TestExecutor::build(2, COUNTER_SOURCE).await;

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
        let executor = TestExecutor::build(2, "export {}").await;
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
        let executor = TestExecutor::build(1, COUNTER_SOURCE).await;
        let nested = executor.clone();

        assert_eq!(
            tokio::time::timeout(
                Duration::from_secs(1),
                executor.execute(move |context| Box::pin(async move {
                    context
                        .guest()
                        .scope(async move |scope| {
                            let module = context.module().bind(&scope)?;
                            let first = module
                                .function("increment")?
                                .call::<_, i32>(())?;
                            let second = nested
                                .execute(|context| {
                                    Box::pin(async move {
                                        context
                                            .module()
                                            .function("increment")
                                            .await?
                                            .call::<_, i32>(())
                                            .await
                                    })
                                })
                                .await
                                .map_err(|error| {
                                    guestjs::errors::Error::unexpected(error.to_string())
                                })?;

                            Ok((first, second))
                        })
                        .await
                })),
            )
            .await
            .unwrap()
            .unwrap(),
            (1, 2),
        );

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn nested_execution_awaits_guest_promise() {
        let executor = TestExecutor::build(1, COUNTER_SOURCE).await;
        let nested = executor.clone();

        assert_eq!(
            tokio::time::timeout(
                Duration::from_secs(1),
                executor.execute(move |context| Box::pin(async move {
                    context
                        .guest()
                        .scope(async move |_scope| {
                            nested
                                .execute(|context| {
                                    Box::pin(async move {
                                        context
                                            .module()
                                            .function("incrementAsync")
                                            .await?
                                            .call::<_, Promise<i32>>(())
                                            .await?
                                            .await
                                    })
                                })
                                .await
                                .map_err(|error| {
                                    guestjs::errors::Error::unexpected(error.to_string())
                                })
                        })
                        .await
                })),
            )
            .await
            .unwrap()
            .unwrap(),
            1,
        );

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn nested_guest_error_preserves_worker() {
        let executor = TestExecutor::build(1, COUNTER_SOURCE).await;
        let nested = executor.clone();
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            executor.execute(move |context| {
                Box::pin(async move {
                    context
                        .guest()
                        .scope(async move |_scope| {
                            Ok(nested
                                .execute(|context| {
                                    Box::pin(async move {
                                        context
                                            .module()
                                            .function("fail")
                                            .await?
                                            .call::<_, ()>(())
                                            .await
                                    })
                                })
                                .await)
                        })
                        .await
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
    async fn supports_three_levels_of_reentry() {
        let executor = TestExecutor::build(1, COUNTER_SOURCE).await;

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
    async fn rejects_shutdown_from_own_worker() {
        let executor = TestExecutor::build(1, "export {}").await;
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
    async fn external_and_local_executions_can_be_spawned() {
        let executor = TestExecutor::build(1, COUNTER_SOURCE).await;

        assert_eq!(
            tokio::spawn(TestExecutor::increment(&executor))
                .await
                .unwrap()
                .unwrap(),
            1,
        );

        let nested = executor.clone();

        assert_eq!(
            tokio::time::timeout(
                Duration::from_secs(1),
                executor.execute(move |_context| Box::pin(async move {
                    Ok(tokio::spawn(TestExecutor::increment(&nested)).await)
                })),
            )
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .unwrap(),
            2,
        );

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn caller_cancellation_shuts_down_executor() {
        let cancellation = CancellationToken::new();
        let executor = Executor::builder("test.ts", "export {}")
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
    async fn shutdown_interrupts_active_execution_and_rejects_later_work() {
        let executor = TestExecutor::build(1, "export {}").await;
        let started = Arc::new(Notify::new());
        let execution = executor.execute({
            let started = started.clone();

            move |context| {
                Box::pin(async move {
                    started.notify_one();
                    context
                        .guest()
                        .eval::<()>("while (true) {}")
                        .await
                })
            }
        });
        let task = tokio::spawn(execution);

        started.notified().await;
        executor.shutdown().await.unwrap();

        assert!(
            tokio::time::timeout(Duration::from_secs(1), task)
                .await
                .unwrap()
                .unwrap()
                .is_err()
        );
        assert!(matches!(
            executor
                .execute(|_context| Box::pin(async move { Ok(()) }))
                .await,
            Err(Error::ExecutorShutdown),
        ));
    }
}
