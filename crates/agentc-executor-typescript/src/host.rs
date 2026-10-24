// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{future::Future, sync::mpsc::sync_channel};

use tokio::runtime::Handle;

use crate::errors::Error;

#[derive(Clone)]
pub struct HostRuntime {
    handle: Handle,
}

impl HostRuntime {
    pub fn new(handle: Handle) -> Self {
        HostRuntime { handle }
    }

    pub fn current() -> Result<Self, Error> {
        Handle::try_current()
            .map(Self::new)
            .map_err(|_| Error::no_host_runtime())
    }

    pub fn block_on<F>(&self, future: F) -> Result<F::Output, Error>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (sender, receiver) = sync_channel(1);

        drop(self.handle.spawn(async move {
            let _ = sender.send(future.await);
        }));

        receiver
            .recv()
            .map_err(|_| Error::host_runtime_stopped())
    }
}

impl From<Handle> for HostRuntime {
    fn from(handle: Handle) -> Self {
        HostRuntime::new(handle)
    }
}

#[cfg(test)]
mod tests {
    use guestjs::{
        bundle::Bundle,
        handle::protocol::{CallableProtocol, ObjectProtocol},
        host::module::ModuleDeclaration,
        quickjs::QuickJs,
    };

    use crate::{errors::Error, executor::Executor, host::HostRuntime};

    const HOST_SOURCE: &str = r#"
import { value } from "test:host";

export function read() {
    return value();
}
"#;

    #[tokio::test]
    async fn block_on_returns_the_future_output() {
        let runtime = HostRuntime::current().unwrap();

        assert_eq!(
            tokio::task::spawn_blocking(move || runtime.block_on(async { 42_i32 }))
                .await
                .unwrap()
                .unwrap(),
            42,
        );
    }

    #[tokio::test]
    async fn block_on_from_inside_a_worker_thread_does_not_panic() {
        let runtime = HostRuntime::current().unwrap();
        let executor = Executor::<QuickJs>::builder("host")
            .bundle(Bundle::single("host.ts", HOST_SOURCE).unwrap())
            .workers(1)
            .configure(move |builder| {
                let runtime = runtime.clone();

                Ok(builder.bind(ModuleDeclaration::<QuickJs>::new("test:host").function(
                    "value",
                    move |_context, _args| {
                        runtime
                            .block_on(async {
                                tokio::task::yield_now().await;
                                42_i32
                            })
                            .map_err(|error| guestjs::errors::Error::unexpected(error.to_string()))
                    },
                )))
            })
            .build()
            .await
            .unwrap();

        assert_eq!(
            executor
                .execute(|context| Box::pin(async move {
                    context
                        .module()
                        .function("read")?
                        .call::<_, i32>(())
                }))
                .await
                .unwrap(),
            42,
        );

        executor.shutdown().await.unwrap();
    }

    #[test]
    fn block_on_reports_a_stopped_runtime() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let host = HostRuntime::from(runtime.handle().clone());

        drop(runtime);

        assert!(matches!(host.block_on(async { 42_i32 }), Err(Error::HostRuntimeStopped),));
    }

    #[test]
    fn current_fails_outside_a_runtime() {
        assert!(matches!(HostRuntime::current(), Err(Error::NoHostRuntime),));
    }
}
