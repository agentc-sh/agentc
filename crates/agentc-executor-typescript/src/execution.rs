// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::error::Error;

/// A pending typed operation submitted to an executor.
#[must_use = "execution results must be awaited"]
pub struct Execution<T> {
    future: Pin<Box<dyn Future<Output = Result<T, Error>> + Send>>,
}

impl<T> Execution<T> {
    pub(crate) fn new<F>(future: F) -> Self
    where
        F: Future<Output = Result<T, Error>> + Send + 'static,
    {
        Self { future: Box::pin(future) }
    }

    pub(crate) fn ready(result: Result<T, Error>) -> Self
    where
        T: Send + 'static,
    {
        Self::new(async move { result })
    }
}

impl<T> Future for Execution<T> {
    type Output = Result<T, Error>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.future.as_mut().poll(context)
    }
}

#[cfg(test)]
mod tests {
    use crate::{error::Error, execution::Execution};

    #[tokio::test]
    async fn forwards_successful_result() {
        assert_eq!(
            Execution::new(async { Ok(42) })
                .await
                .unwrap(),
            42
        );
    }

    #[tokio::test]
    async fn forwards_executor_error() {
        assert!(matches!(
            Execution::<()>::ready(Err(Error::executor_shutdown())).await,
            Err(Error::ExecutorShutdown),
        ));
    }
}
