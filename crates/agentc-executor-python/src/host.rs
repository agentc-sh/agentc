// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{future::Future, sync::mpsc::sync_channel};

use tokio::runtime::Handle;

use crate::error::Error;

/// A cloneable capture of the application's Tokio runtime handle.
#[derive(Clone)]
pub struct HostRuntime {
    handle: Handle,
}

impl HostRuntime {
    /// Creates a host runtime bridge from an explicit Tokio handle.
    pub fn new(handle: Handle) -> Self {
        Self { handle }
    }

    /// Captures the currently entered Tokio runtime handle.
    pub fn current() -> Result<Self, Error> {
        Handle::try_current()
            .map(Self::new)
            .map_err(|_| Error::no_host_runtime())
    }

    /// Runs a future on the application runtime and blocks the caller for its output.
    pub fn block_on<F>(&self, future: F) -> Result<F::Output, Error>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (sender, receiver) = sync_channel(1);

        drop(self.handle.spawn(async move {
            let _ = sender.send(future.await);
        }));

        receiver.recv().map_err(|_| Error::host_runtime_stopped())
    }
}

impl From<Handle> for HostRuntime {
    fn from(handle: Handle) -> Self {
        Self::new(handle)
    }
}

#[cfg(test)]
mod tests {
    use guestpy::{bundle::Bundle, host::module::ModuleSpec, rustpython::RustPython};

    use crate::{error::Error, executor::Executor, host::HostRuntime};

    const HOST_SOURCE: &str = "\
def read_host_value():
    import test_host
    return test_host.value()
";

    #[tokio::test]
    async fn block_on_returns_the_future_output() {
        let runtime = HostRuntime::current().unwrap();

        assert_eq!(
            tokio::task::spawn_blocking(move || runtime.block_on(async { 42_i64 }))
                .await
                .unwrap()
                .unwrap(),
            42,
        );
    }

    #[test]
    fn current_fails_outside_a_runtime() {
        assert!(matches!(HostRuntime::current(), Err(Error::NoHostRuntime)));
    }

    #[test]
    fn block_on_reports_a_stopped_runtime() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let host = HostRuntime::from(runtime.handle().clone());

        drop(runtime);

        assert!(matches!(
            host.block_on(async { 42_i64 }),
            Err(Error::HostRuntimeStopped),
        ));
    }

    #[tokio::test]
    async fn block_on_from_python_worker_calls_async_host_service() {
        let runtime = HostRuntime::current().unwrap();
        let executor = Executor::<RustPython>::builder("test_host_component")
            .bundle(Bundle::single("test_host_component", HOST_SOURCE).unwrap())
            .workers(1)
            .configure(move |builder| {
                let runtime = runtime.clone();

                builder.bind(
                    ModuleSpec::<RustPython>::new("test_host").function("value", move |_enter, _args| {
                        runtime
                            .block_on(async {
                                tokio::task::yield_now().await;
                                42_i64
                            })
                            .map_err(|error| guestpy::errors::Error::unexpected(error.to_string()))
                    }),
                )
            })
            .build()
            .await
            .unwrap();

        assert_eq!(
            executor
                .execute(|context| Box::pin(async move {
                    context
                        .module()
                        .function("read_host_value")?
                        .call::<_, i64>(())
                }))
                .await
                .unwrap(),
            42,
        );

        executor.shutdown().await.unwrap();
    }
}
