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
    use std::{panic::resume_unwind, rc::Rc};

    use guestpy::{bundle::Bundle, runtime::Runtime, rustpython::RustPython};
    use tokio::{
        runtime::Builder,
        task::{LocalSet, yield_now},
    };

    use crate::{
        context::Context,
        errors::Error,
        job::TypedJob,
        worker::{ExecutorId, WorkerId, WorkerThread},
    };

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

    struct TestWorker;

    impl TestWorker {
        fn run<F, Fut>(operation: F)
        where
            F: FnOnce() -> Fut + Send + 'static,
            Fut: Future<Output = ()> + 'static,
        {
            // Re-raising on the test thread fails the test with the worker's original panic.
            if let Err(panic) = WorkerThread::new(ExecutorId::next(), WorkerId::new(0))
                .spawn(|| {
                    Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap()
                        .block_on(LocalSet::new().run_until(operation()))
                })
                .unwrap()
                .join()
            {
                resume_unwind(panic);
            }
        }
    }

    #[test]
    fn executes_local_future_with_typed_result() {
        TestWorker::run(|| async {
            let (job, response) = TypedJob::prepare(|_context| {
                Box::pin(async move {
                    let value = Rc::new(42);

                    yield_now().await;

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
        });
    }

    #[test]
    fn converts_guest_error() {
        TestWorker::run(|| async {
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
        });
    }
}
