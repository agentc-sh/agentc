// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use futures::future::LocalBoxFuture;

use crate::{
    backend::ExecutorBackend,
    context::Context,
    execution::Execution,
    executor::ExecutorInner,
    worker::{ExecutionContext, WorkerId},
};

/// A cloneable affinity handle that routes operations to one worker.
pub struct WorkerLease<B: ExecutorBackend> {
    executor: Arc<ExecutorInner<B>>,
    worker: WorkerId,
}

impl<B: ExecutorBackend> Clone for WorkerLease<B> {
    fn clone(&self) -> Self {
        Self {
            executor: self.executor.clone(),
            worker: self.worker,
        }
    }
}

impl<B: ExecutorBackend> WorkerLease<B> {
    pub(crate) fn new(executor: Arc<ExecutorInner<B>>, worker: WorkerId) -> Self {
        Self { executor, worker }
    }

    /// Executes a typed GuestJS operation on the lease's worker.
    pub fn execute<F, T>(&self, operation: F) -> Execution<T>
    where
        F: for<'a> FnOnce(&'a Context<B>) -> LocalBoxFuture<'a, Result<T, guestjs::errors::Error>>
            + Send
            + 'static,
        T: Send + 'static,
    {
        match ExecutionContext::for_worker::<B>(self.executor.id(), self.worker) {
            Some(context) => self
                .executor
                .dispatch_local(context, operation),
            None => self
                .executor
                .dispatch(self.worker, operation),
        }
    }

    /// Returns whether executor shutdown has begun.
    pub fn is_shutdown(&self) -> bool {
        self.executor.is_shutdown()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use guestjs::{
        bundle::Bundle,
        handle::protocol::{CallableProtocol, ObjectProtocol},
        quickjs::QuickJs,
    };

    use crate::{errors::Error, execution::Execution, executor::Executor, lease::WorkerLease};

    const COMPONENT_SOURCE: &str = r#"
let count = 0;

export function increment() {
    count += 1;
    return count;
}
"#;

    struct TestLease;

    impl TestLease {
        fn increment(lease: &WorkerLease<QuickJs>) -> Execution<i32> {
            lease.execute(|context| {
                Box::pin(async move {
                    context
                        .module()
                        .function("increment")?
                        .call::<_, i32>(())
                })
            })
        }
    }

    #[tokio::test]
    async fn preserves_worker_affinity() {
        let executor = Executor::<QuickJs>::builder("test_component")
            .bundle(Bundle::single("test_component.ts", COMPONENT_SOURCE).unwrap())
            .workers(2)
            .build()
            .await
            .unwrap();
        let lease = executor.lease();

        assert_eq!(
            [
                TestLease::increment(&lease)
                    .await
                    .unwrap(),
                TestLease::increment(&lease)
                    .await
                    .unwrap(),
                TestLease::increment(&lease)
                    .await
                    .unwrap(),
            ],
            [1, 2, 3],
        );

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn executes_locally_on_selected_worker() {
        let executor = Executor::<QuickJs>::builder("test_component")
            .bundle(Bundle::single("test_component.ts", COMPONENT_SOURCE).unwrap())
            .workers(1)
            .build()
            .await
            .unwrap();
        let lease = executor.lease();
        let nested = lease.clone();

        assert_eq!(
            tokio::time::timeout(
                Duration::from_secs(1),
                lease.execute(move |context| {
                    Box::pin(async move {
                        let first = context
                            .module()
                            .function("increment")?
                            .call::<_, i32>(())?;
                        let second = nested
                            .execute(|context| {
                                Box::pin(async move {
                                    context
                                        .module()
                                        .function("increment")?
                                        .call::<_, i32>(())
                                })
                            })
                            .await
                            .map_err(|error| {
                                guestjs::errors::Error::unexpected(error.to_string())
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
    async fn reports_executor_shutdown() {
        let executor = Executor::<QuickJs>::builder("test_component")
            .bundle(Bundle::single("test_component.ts", COMPONENT_SOURCE).unwrap())
            .workers(1)
            .build()
            .await
            .unwrap();
        let lease = executor.lease();

        executor.shutdown().await.unwrap();

        assert!(lease.is_shutdown());
        assert!(matches!(TestLease::increment(&lease).await, Err(Error::ExecutorShutdown),));
    }
}
