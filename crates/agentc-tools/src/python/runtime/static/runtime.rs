// SPDX-FileCopyrightText: 2026 Timothy Pogue
//
// SPDX-License-Identifier: LicenseRef-Proprietary

use serde_json::Value;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::python::runtime::{errors::RuntimeError, protocol::Command, traits::Runtime};

use super::interpreter::StaticRuntimeWorker;

pub struct StaticRuntime {
    workers: Vec<StaticRuntimeWorker>,
    next: AtomicUsize,
}

impl StaticRuntime {
    pub fn new(
        num_interpreters: usize,
        channel_size: usize,
        shutdown: CancellationToken,
    ) -> Result<Self, RuntimeError> {
        let mut workers = Vec::with_capacity(num_interpreters);

        for _ in 0..num_interpreters {
            workers.push(StaticRuntimeWorker::new(channel_size, shutdown.clone())?);
        }

        Ok(Self { workers, next: AtomicUsize::new(0) })
    }

    pub fn builder() -> StaticRuntimeBuilder {
        StaticRuntimeBuilder::new()
    }

    fn worker(&self) -> &StaticRuntimeWorker {
        &self.workers[self
            .next
            .fetch_add(1, Ordering::Relaxed)
            % self.workers.len()]
    }

    pub fn send_command(&self, command: Command) -> oneshot::Receiver<Result<Value, RuntimeError>> {
        self.worker().send_command(command)
    }

    pub fn broadcast_command(
        &self,
        command: Command,
    ) -> oneshot::Receiver<Result<Value, RuntimeError>> {
        self.workers
            .iter()
            .map(|worker| worker.send_command(command.clone()))
            .next_back()
            .expect("no workers available for broadcast")
    }

    pub fn close(self) -> Result<(), RuntimeError> {
        let mut errors = Vec::new();

        for worker in self.workers {
            if let Err(err) = worker.close() {
                errors.push(err);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(RuntimeError::shutdown(format!("failed to close all workers: {:?}", errors)))
        }
    }
}

impl Runtime for StaticRuntime {
    fn dispatch(&self, command: Command) -> oneshot::Receiver<Result<Value, RuntimeError>> {
        match command {
            Command::Import { .. } | Command::SetGlobal { .. } => self.broadcast_command(command),
            _ => self.send_command(command),
        }
    }
}

pub struct StaticRuntimeBuilder {
    num_interpreters: usize,
    channel_size: usize,
    shutdown: CancellationToken,
}

impl StaticRuntimeBuilder {
    pub fn new() -> Self {
        Self {
            num_interpreters: 4,
            channel_size: 32,
            shutdown: CancellationToken::new(),
        }
    }

    /// Set the number of worker threads (interpreters) to spawn. Default is 4.
    pub fn num_interpreters(mut self, num_interpreters: usize) -> Self {
        self.num_interpreters = num_interpreters;
        self
    }

    /// Set the channel size for each worker thread. Default is 32.
    pub fn channel_size(mut self, channel_size: usize) -> Self {
        self.channel_size = channel_size;
        self
    }

    /// Set the shutdown token for all interpreter threads in this runtime.
    ///
    /// When the token is cancelled, each thread exits its command loop after
    /// finishing any in-progress work.
    pub fn shutdown(mut self, token: CancellationToken) -> Self {
        self.shutdown = token;
        self
    }

    /// Build the [`StaticRuntime`], spawning worker threads and initializing interpreters.
    pub fn build(self) -> Result<StaticRuntime, RuntimeError> {
        StaticRuntime::new(self.num_interpreters, self.channel_size, self.shutdown)
    }
}

impl Default for StaticRuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::python::runtime::{protocol::FunctionArgs, traits::RuntimeExt};
    use serde_json::json;

    fn runtime() -> StaticRuntime {
        StaticRuntime::builder()
            .num_interpreters(1)
            .channel_size(32)
            .build()
            .expect("failed to create StaticRuntime")
    }

    #[tokio::test]
    async fn eval_simple_expr() {
        let runtime = runtime();
        assert_eq!(
            runtime
                .eval::<i32>("1 + 2")
                .await
                .expect("eval failed"),
            3
        );
        runtime.close().expect("close failed");
    }

    #[tokio::test]
    async fn exec_simple_stmt() {
        let runtime = runtime();
        runtime
            .exec("x = 5")
            .await
            .expect("exec failed");
        assert_eq!(
            runtime
                .eval::<i32>("x")
                .await
                .expect("eval failed"),
            5
        );
        runtime.close().expect("close failed");
    }

    #[tokio::test]
    async fn set_global() {
        let runtime = runtime();
        runtime
            .set_global("y", 10)
            .await
            .expect("set_global failed");
        assert_eq!(
            runtime
                .eval::<i32>("y")
                .await
                .expect("eval failed"),
            10
        );
        runtime.close().expect("close failed");
    }

    #[tokio::test]
    async fn get_global() {
        let runtime = runtime();
        runtime
            .exec("z = 20")
            .await
            .expect("exec failed");
        assert_eq!(
            runtime
                .get_global::<i32>("z")
                .await
                .expect("get_global failed"),
            20
        );
        runtime.close().expect("close failed");
    }

    #[tokio::test]
    async fn import_module() {
        let runtime = runtime();
        assert!(
            !runtime
                .import("sys")
                .await
                .expect("import failed")
                .is_empty()
        );
        runtime.close().expect("close failed");
    }

    #[tokio::test]
    async fn call_function() {
        let runtime = runtime();

        runtime
            .exec(
                r#"
import sys, types

def add(a, b):
    return a + b

_m = types.ModuleType('math_fn')
_m.add = add
sys.modules['math_fn'] = _m
                "#,
            )
            .await
            .expect("exec failed");

        let result: i32 = runtime
            .call_function(
                "math_fn",
                "add",
                FunctionArgs::new()
                    .positional(json!(3))
                    .positional(json!(4)),
            )
            .await
            .expect("call_function failed");

        assert_eq!(result, 7);
        runtime.close().expect("close failed");
    }

    #[tokio::test]
    async fn call_method() {
        let runtime = runtime();

        runtime
            .exec(
                r#"
import sys, types

class Counter:
    value = 0

    def increment(self, args, emit=None):
        return self.value + args.get('n', 1)

_m = types.ModuleType('counter_mod')
_m.Counter = Counter
sys.modules['counter_mod'] = _m
                "#,
            )
            .await
            .expect("exec failed");

        let result: i32 = runtime
            .call_method(
                "counter_mod",
                "Counter",
                "increment",
                FunctionArgs::new().positional(json!({"n": 5})),
            )
            .await
            .expect("call_method failed");

        assert_eq!(result, 5);
        runtime.close().expect("close failed");
    }
}
