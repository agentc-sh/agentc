// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::rc::Rc;

use futures::future::LocalBoxFuture;
use tokio::sync::oneshot;

use crate::{context::Context, error::Error};

pub(crate) trait Job: Send {
    fn execute(self: Box<Self>, context: Rc<Context>) -> LocalBoxFuture<'static, ()>;
}

pub(crate) struct TypedJob<F, T> {
    operation: F,
    response: oneshot::Sender<Result<T, Error>>,
}

impl<F, T> TypedJob<F, T> {
    pub(crate) fn prepare(operation: F) -> (Box<dyn Job>, oneshot::Receiver<Result<T, Error>>)
    where
        F: for<'a> FnOnce(&'a Context) -> LocalBoxFuture<'a, Result<T, guestjs::errors::Error>>
            + Send
            + 'static,
        T: Send + 'static,
    {
        let (response, receiver) = oneshot::channel();
        (Box::new(Self { operation, response }), receiver)
    }
}

impl<F, T> Job for TypedJob<F, T>
where
    F: for<'a> FnOnce(&'a Context) -> LocalBoxFuture<'a, Result<T, guestjs::errors::Error>>
        + Send
        + 'static,
    T: Send + 'static,
{
    fn execute(self: Box<Self>, context: Rc<Context>) -> LocalBoxFuture<'static, ()> {
        let Self { operation, response } = *self;

        Box::pin(async move {
            let _ = response.send(
                operation(context.as_ref())
                    .await
                    .map_err(Error::guest),
            );
        })
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use guestjs::runtime::Runtime;

    use crate::{context::Context, error::Error, job::TypedJob};

    struct TestContext;

    impl TestContext {
        async fn build() -> Rc<Context> {
            let runtime = Runtime::builder()
                .build()
                .await
                .unwrap();
            let guest = runtime.guest().build().await.unwrap();
            let module = guest
                .guest_module("test.js", "export const value = 42;")
                .await
                .unwrap();

            Rc::new(Context::new(runtime, guest, module))
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn executes_local_future_with_typed_result() {
        let (job, response) = TypedJob::prepare(|_context| {
            Box::pin(async move {
                let value = Rc::new(42);

                tokio::task::yield_now().await;

                Ok(*value)
            })
        });

        job.execute(TestContext::build().await)
            .await;

        assert_eq!(response.await.unwrap().unwrap(), 42);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn converts_guest_error() {
        let (job, response) = TypedJob::<_, ()>::prepare(|_context| {
            Box::pin(async move { Err(guestjs::errors::Error::guest_exception("failed")) })
        });

        job.execute(TestContext::build().await)
            .await;

        assert!(matches!(
            response.await.unwrap(),
            Err(Error::Guest(guestjs::errors::Error::GuestException { .. })),
        ));
    }
}
