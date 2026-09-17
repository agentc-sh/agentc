// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::rc::Rc;

use futures::future::LocalBoxFuture;
use tokio::sync::oneshot;

use crate::{backend::ExecutorBackend, context::Context, errors::Error};

pub(crate) trait Job<B: ExecutorBackend>: Send {
    fn execute(self: Box<Self>, context: Rc<Context<B>>) -> LocalBoxFuture<'static, ()>;
}

pub(crate) struct TypedJob<F, T> {
    operation: F,
    response: oneshot::Sender<Result<T, Error>>,
}

impl<F, T> TypedJob<F, T> {
    pub(crate) fn prepare<B>(operation: F) -> (Box<dyn Job<B>>, oneshot::Receiver<Result<T, Error>>)
    where
        B: ExecutorBackend,
        F: for<'a> FnOnce(&'a Context<B>) -> LocalBoxFuture<'a, Result<T, guestpy::errors::Error>>
            + Send
            + 'static,
        T: Send + 'static,
    {
        let (response, receiver) = oneshot::channel();

        (Box::new(Self { operation, response }), receiver)
    }
}

impl<B, F, T> Job<B> for TypedJob<F, T>
where
    B: ExecutorBackend,
    F: for<'a> FnOnce(&'a Context<B>) -> LocalBoxFuture<'a, Result<T, guestpy::errors::Error>>
        + Send
        + 'static,
    T: Send + 'static,
{
    fn execute(self: Box<Self>, context: Rc<Context<B>>) -> LocalBoxFuture<'static, ()> {
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

    use guestpy::{bundle::Bundle, runtime::Runtime, rustpython::RustPython};

    use crate::{context::Context, errors::Error, job::TypedJob};

    struct TestContext;

    impl TestContext {
        fn build() -> Rc<Context<RustPython>> {
            let runtime = Runtime::<RustPython>::builder()
                .bundle(Bundle::single("test_component", "value = 42\n").unwrap())
                .build()
                .unwrap();
            let guest = runtime.guest().build().unwrap();
            let module = guest.import("test_component").unwrap();

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

        let context = TestContext::build();

        job.execute(context.clone()).await;

        assert_eq!(response.await.unwrap().unwrap(), 42);

        let Ok(context) = Rc::try_unwrap(context) else {
            panic!("test retained the worker context");
        };

        context.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn converts_guest_error() {
        let (job, response) = TypedJob::<_, ()>::prepare(|_context| {
            Box::pin(async move { Err(guestpy::errors::Error::unexpected("failed")) })
        });

        let context = TestContext::build();

        job.execute(context.clone()).await;

        assert!(matches!(response.await.unwrap(), Err(Error::Guest(_))));

        let Ok(context) = Rc::try_unwrap(context) else {
            panic!("test retained the worker context");
        };

        context.shutdown().await.unwrap();
    }
}
