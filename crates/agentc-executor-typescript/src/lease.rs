// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use futures::future::LocalBoxFuture;

use crate::{
    context::Context,
    execution::Execution,
    executor::ExecutorInner,
    worker::{ExecutionContext, WorkerId},
};

/// A cloneable affinity handle that routes operations to one worker.
#[derive(Clone)]
pub struct WorkerLease {
    executor: Arc<ExecutorInner>,
    worker: WorkerId,
}

impl WorkerLease {
    pub(crate) fn new(executor: Arc<ExecutorInner>, worker: WorkerId) -> Self {
        Self { executor, worker }
    }

    /// Executes a typed GuestJS operation on the lease's worker.
    pub fn execute<F, T>(&self, operation: F) -> Execution<T>
    where
        F: for<'a> FnOnce(&'a Context) -> LocalBoxFuture<'a, Result<T, guestjs::errors::Error>>
            + Send
            + 'static,
        T: Send + 'static,
    {
        match ExecutionContext::for_worker(self.executor.id, self.worker) {
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

    use crate::{execution::Execution, executor::Executor, lease::WorkerLease};

    const COUNTER_SOURCE: &str = r#"
let count = 0;

export function increment() {
    count += 1;
    return count;
}
"#;

    struct TestLease;

    impl TestLease {
        fn increment(lease: &WorkerLease) -> Execution<i32> {
            lease.execute(|context| {
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
    }

    #[tokio::test]
    async fn preserves_worker_affinity() {
        let executor = Executor::builder("test.ts", COUNTER_SOURCE)
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
        let executor = Executor::builder("test.ts", COUNTER_SOURCE)
            .workers(1)
            .build()
            .await
            .unwrap();
        let lease = executor.lease();
        let nested = lease.clone();

        assert_eq!(
            tokio::time::timeout(
                Duration::from_secs(1),
                lease.execute(move |context| Box::pin(async move {
                    let first = context
                        .module()
                        .function("increment")
                        .await?
                        .call::<_, i32>(())
                        .await?;
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
                        .map_err(|error| guestjs::errors::Error::unexpected(error.to_string()))?;

                    Ok((first, second))
                })),
            )
            .await
            .unwrap()
            .unwrap(),
            (1, 2),
        );

        executor.shutdown().await.unwrap();
    }
}
