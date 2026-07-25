// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rustpython_vm::{
    AsObject, Interpreter, Settings, VirtualMachine,
    builtins::{PyBaseExceptionRef, PyModuleDef},
    compiler::Mode,
    convert::TryFromObject,
    frozen::FrozenModule,
    function::FuncArgs,
    object::PyObjectRef,
    py_serde,
    scope::Scope,
};
use serde::{Serialize, de::DeserializeOwned, de::IntoDeserializer};
use serde_json::{Value, from_value, to_value, value::Serializer};
use std::{
    sync::mpsc::RecvTimeoutError,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::python::runtime::{
    errors::RuntimeError,
    protocol::{ArgValue, Command, FunctionArgs},
    traits::Runtime,
};

/// Type alias for a native module factory function. Receives the interpreter [`Context`]
/// and returns a reference to the module's static definition. Using a factory rather than
/// a pre-resolved `&'static PyModuleDef` means the ctx is only applied inside the worker
/// thread, keeping the builder API arg-free at the callsite.
pub type NativeModuleDef = fn(&rustpython_vm::Context) -> &'static PyModuleDef;

struct InterpreterMessage {
    command: Command,
    tx: oneshot::Sender<Result<Value, RuntimeError>>,
}

/// The interpreter context that lives on the worker thread.
///
/// Owns the RustPython [`Interpreter`] and the global [`Scope`]. All command dispatch
/// and Python object conversion happens through methods on this struct so that the
/// interpreter context is never accessed outside the worker thread.
struct InterpreterContext {
    interpreter: Interpreter,
    scope: Scope,
}

impl InterpreterContext {
    fn new(interpreter: Interpreter) -> Self {
        let scope = interpreter.enter(|vm| vm.new_scope_with_builtins());
        Self { interpreter, scope }
    }

    fn enter<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&VirtualMachine) -> R,
    {
        self.interpreter.enter(|vm| f(vm))
    }

    fn dispatch(&self, vm: &VirtualMachine, command: Command) -> Result<Value, RuntimeError> {
        match command {
            Command::Eval { source } => Self::deserialize_py(
                vm,
                vm.run_code_obj(
                    vm.compile(&source, Mode::Eval, "<eval>".to_string())
                        .map_err(|e| RuntimeError::python(e.to_string()))?,
                    self.scope.clone(),
                )
                .map_err(|e| RuntimeError::python(Self::format_exc(vm, e)))?,
            ),
            Command::Exec { source } => {
                vm.run_code_obj(
                    vm.compile(&source, Mode::Exec, "<exec>".to_string())
                        .map_err(|e| RuntimeError::python(e.to_string()))?,
                    self.scope.clone(),
                )
                .map_err(|e| RuntimeError::python(Self::format_exc(vm, e)))?;

                Ok(Value::Null)
            }
            Command::SetGlobal { name, value } => {
                self.scope
                    .globals
                    .set_item(name.as_str(), Self::serialize_py(vm, value)?, vm)
                    .map_err(|e| RuntimeError::python(Self::format_exc(vm, e)))?;

                Ok(Value::Null)
            }
            Command::GetGlobal { name } => Self::deserialize_py(
                vm,
                self.scope
                    .globals
                    .get_item(name.as_str(), vm)
                    .map_err(|e| RuntimeError::python(Self::format_exc(vm, e)))?,
            ),
            Command::ListGlobals => to_value(
                self.scope
                    .globals
                    .keys_vec()
                    .into_iter()
                    .filter_map(|k| String::try_from_object(vm, k).ok())
                    .collect::<Vec<_>>(),
            )
            .map_err(RuntimeError::serialize),
            Command::Import { name } => {
                let module = vm
                    .import(&vm.ctx.new_str(name.clone()), 0)
                    .map_err(|e| RuntimeError::python(Self::format_exc(vm, e)))?;

                let attributes = module
                    .as_object()
                    .dict()
                    .ok_or_else(|| RuntimeError::python("imported module has no __dict__"))?
                    .keys_vec()
                    .into_iter()
                    .filter_map(|k| String::try_from_object(vm, k).ok())
                    .collect::<Vec<_>>();

                self.scope
                    .globals
                    .set_item(name.as_str(), module, vm)
                    .map_err(|e| RuntimeError::python(Self::format_exc(vm, e)))?;

                to_value(attributes).map_err(RuntimeError::serialize)
            }
            Command::CallFunction { module, name, args } => {
                let module_obj = match self
                    .scope
                    .globals
                    .get_item_opt(module.as_str(), vm)
                    .map_err(|e| RuntimeError::python(Self::format_exc(vm, e)))?
                {
                    Some(obj) => obj,
                    None => vm
                        .import(&vm.ctx.new_str(module.clone()), 0)
                        .map_err(|e| RuntimeError::python(Self::format_exc(vm, e)))?,
                };

                let func = module_obj
                    .get_attr(&vm.ctx.new_str(name), vm)
                    .map_err(|e| RuntimeError::python(Self::format_exc(vm, e)))?;

                let result = func
                    .call(Self::args_to_py(vm, args)?, vm)
                    .map_err(|e| RuntimeError::python(Self::format_exc(vm, e)))?;

                Self::deserialize_py(vm, result)
            }
            Command::CallMethod { module, class_name, method, args } => {
                let module_obj = match self
                    .scope
                    .globals
                    .get_item_opt(module.as_str(), vm)
                    .map_err(|e| RuntimeError::python(Self::format_exc(vm, e)))?
                {
                    Some(obj) => obj,
                    None => vm
                        .import(&vm.ctx.new_str(module.clone()), 0)
                        .map_err(|e| RuntimeError::python(Self::format_exc(vm, e)))?,
                };

                let cls = module_obj
                    .get_attr(&vm.ctx.new_str(class_name), vm)
                    .map_err(|e| RuntimeError::python(Self::format_exc(vm, e)))?;

                let instance = cls
                    .call((), vm)
                    .map_err(|e| RuntimeError::python(Self::format_exc(vm, e)))?;

                let method_attr = instance
                    .get_attr(&vm.ctx.new_str(method), vm)
                    .map_err(|e| RuntimeError::python(Self::format_exc(vm, e)))?;

                let result = method_attr
                    .call(Self::args_to_py(vm, args)?, vm)
                    .map_err(|e| RuntimeError::python(Self::format_exc(vm, e)))?;

                Self::deserialize_py(vm, result)
            }
        }
    }

    /// Convert a [`Value`] into a RustPython object via `py_serde`.
    fn serialize_py<T: Serialize>(
        vm: &VirtualMachine,
        value: T,
    ) -> Result<PyObjectRef, RuntimeError> {
        py_serde::deserialize(
            vm,
            to_value(value)
                .map_err(RuntimeError::serialize)?
                .into_deserializer(),
        )
        .map_err(|_| RuntimeError::python("failed to serialize value into Python object"))
    }

    /// Convert a RustPython object into a `T` via `py_serde` + JSON.
    fn deserialize_py<T: DeserializeOwned>(
        vm: &VirtualMachine,
        obj: PyObjectRef,
    ) -> Result<T, RuntimeError> {
        from_value(py_serde::serialize(vm, obj.as_object(), Serializer).map_err(|_| {
            RuntimeError::python("failed to deserialize Python object into JSON value")
        })?)
        .map_err(RuntimeError::serialize)
    }

    /// Format a Python exception into a human-readable string.
    fn format_exc(vm: &VirtualMachine, exc: PyBaseExceptionRef) -> String {
        let mut buf = String::new();
        vm.write_exception(&mut buf, &exc)
            .unwrap_or_default();
        buf
    }

    /// Convert a single [`ArgValue`] into a RustPython object.
    fn arg_to_py(vm: &VirtualMachine, arg: ArgValue) -> Result<PyObjectRef, RuntimeError> {
        match arg {
            ArgValue::Json(value) => Self::serialize_py(vm, value),
            ArgValue::Callable(callable) => {
                let py_fn = vm.new_function(
                    "native_fn",
                    move |func_args: FuncArgs,
                          vm: &VirtualMachine|
                          -> rustpython_vm::PyResult<PyObjectRef> {
                        // Deserialize each positional arg from Python to JSON ArgValue.
                        let positional = func_args
                            .args
                            .into_iter()
                            .map(|obj| {
                                py_serde::serialize(vm, obj.as_object(), Serializer)
                                    .map_err(|_| {
                                        vm.new_runtime_error(
                                            "failed to deserialize positional argument".to_owned(),
                                        )
                                    })
                                    .map(ArgValue::Json)
                            })
                            .collect::<rustpython_vm::PyResult<Vec<_>>>()?;

                        let keyword = func_args
                            .kwargs
                            .into_iter()
                            .map(|(k, obj)| {
                                py_serde::serialize(vm, obj.as_object(), Serializer)
                                    .map_err(|_| {
                                        vm.new_runtime_error(
                                            "failed to deserialize keyword argument".to_owned(),
                                        )
                                    })
                                    .map(|v| (k, ArgValue::Json(v)))
                            })
                            .collect::<rustpython_vm::PyResult<Vec<_>>>()?;

                        let result = callable(FunctionArgs { positional, keyword })
                            .map_err(|e| vm.new_runtime_error(e.to_string()))?;

                        py_serde::deserialize(
                            vm,
                            to_value(result)
                                .map_err(|e| vm.new_runtime_error(e.to_string()))?
                                .into_deserializer(),
                        )
                        .map_err(|_| {
                            vm.new_runtime_error("failed to serialize callable result".to_owned())
                        })
                    },
                );

                Ok(py_fn.into())
            }
        }
    }

    /// Convert a [`FunctionArgs`] into a RustPython [`FuncArgs`].
    fn args_to_py(vm: &VirtualMachine, args: FunctionArgs) -> Result<FuncArgs, RuntimeError> {
        let positional = args
            .positional
            .into_iter()
            .map(|arg| Self::arg_to_py(vm, arg))
            .collect::<Result<Vec<_>, _>>()?;

        let kwargs = args
            .keyword
            .into_iter()
            .map(|(name, arg)| Ok::<_, RuntimeError>((name, Self::arg_to_py(vm, arg)?)))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(FuncArgs {
            args: positional,
            kwargs: kwargs.into_iter().collect(),
        })
    }
}

struct EmbeddedInterpreter {
    frozen: Vec<(&'static str, FrozenModule)>,
    native: Vec<NativeModuleDef>,
    channel_size: usize,
    shutdown: CancellationToken,
}

impl EmbeddedInterpreter {
    fn new(
        frozen: Vec<(&'static str, FrozenModule)>,
        native: Vec<NativeModuleDef>,
        channel_size: usize,
        shutdown: CancellationToken,
    ) -> Self {
        Self { frozen, native, channel_size, shutdown }
    }

    fn spawn(self) -> Result<(mpsc::SyncSender<InterpreterMessage>, JoinHandle<()>), RuntimeError> {
        let (ready_tx, ready_rx) = mpsc::channel();

        let handle = thread::Builder::new()
            // RustPython's frozen stdlib init chain needs more stack than the default 2 MB.
            .stack_size(8 * 1024 * 1024)
            .spawn(move || self.run(ready_tx))
            .map_err(RuntimeError::io)?;

        let msg_tx = ready_rx
            .recv()
            .map_err(|_| RuntimeError::worker_closed())??;

        Ok((msg_tx, handle))
    }

    fn run(
        self,
        ready_tx: mpsc::Sender<Result<mpsc::SyncSender<InterpreterMessage>, RuntimeError>>,
    ) {
        let mut builder = Interpreter::builder(Settings::default());
        let stdlib_defs = rustpython_stdlib::stdlib_module_defs(&builder.ctx);

        builder = builder.add_native_modules(&stdlib_defs);
        builder = builder.add_frozen_modules(rustpython_pylib::FROZEN_STDLIB);

        for factory in &self.native {
            let module = factory(&builder.ctx);
            builder = builder.add_native_module(module);
        }

        builder = builder.add_frozen_modules(self.frozen);

        let ctx = InterpreterContext::new(builder.build());
        let (msg_tx, msg_rx) = mpsc::sync_channel(self.channel_size);

        if ready_tx.send(Ok(msg_tx)).is_err() {
            return;
        }

        ctx.enter(|vm| {
            loop {
                match msg_rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(InterpreterMessage { command, tx: reply }) => {
                        let _ = reply.send(ctx.dispatch(vm, command));
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        if self.shutdown.is_cancelled() {
                            break;
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        });
    }
}

pub struct EmbeddedRuntimeWorker {
    msg_tx: mpsc::SyncSender<InterpreterMessage>,
    handle: JoinHandle<()>,
}

impl EmbeddedRuntimeWorker {
    pub fn new(
        frozen: Vec<(&'static str, FrozenModule)>,
        native: Vec<NativeModuleDef>,
        channel_size: usize,
        shutdown: CancellationToken,
    ) -> Result<Self, RuntimeError> {
        let (tx, handle) =
            EmbeddedInterpreter::new(frozen, native, channel_size, shutdown).spawn()?;
        Ok(Self { msg_tx: tx, handle })
    }

    pub fn close(self) -> Result<(), RuntimeError> {
        drop(self.msg_tx);

        self.handle.join().map_err(|err| {
            err.downcast::<RuntimeError>()
                .map(|e| *e)
                .unwrap_or(RuntimeError::worker_closed())
        })
    }

    fn send_command(&self, command: Command) -> oneshot::Receiver<Result<Value, RuntimeError>> {
        let (reply_tx, reply_rx) = oneshot::channel();

        if let Err(e) = self
            .msg_tx
            .try_send(InterpreterMessage { command, tx: reply_tx })
        {
            let (err, msg) = match e {
                mpsc::TrySendError::Full(m) => (RuntimeError::worker_busy(), m),
                mpsc::TrySendError::Disconnected(m) => (RuntimeError::worker_closed(), m),
            };

            let _ = msg.tx.send(Err(err));
        }

        reply_rx
    }
}

pub struct EmbeddedRuntime {
    workers: Vec<EmbeddedRuntimeWorker>,
    next: AtomicUsize,
}

impl EmbeddedRuntime {
    pub fn new(
        frozen: Vec<(&'static str, FrozenModule)>,
        native: Vec<NativeModuleDef>,
        num_interpreters: usize,
        channel_size: usize,
        shutdown: CancellationToken,
    ) -> Result<Self, RuntimeError> {
        let mut workers = Vec::with_capacity(num_interpreters);

        for _ in 0..num_interpreters {
            workers.push(EmbeddedRuntimeWorker::new(
                frozen.clone(),
                native.clone(),
                channel_size,
                shutdown.clone(),
            )?);
        }

        Ok(Self { workers, next: AtomicUsize::new(0) })
    }

    pub fn builder() -> EmbeddedRuntimeBuilder {
        EmbeddedRuntimeBuilder::new()
    }

    pub fn worker(&self) -> &EmbeddedRuntimeWorker {
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
            .collect::<Vec<_>>()
            .pop()
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

impl Runtime for EmbeddedRuntime {
    fn dispatch(&self, command: Command) -> oneshot::Receiver<Result<Value, RuntimeError>> {
        match command {
            Command::Import { .. } | Command::SetGlobal { .. } => self.broadcast_command(command),
            _ => self.send_command(command),
        }
    }
}

pub struct EmbeddedRuntimeBuilder {
    frozen: Vec<(&'static str, FrozenModule)>,
    native: Vec<NativeModuleDef>,
    num_interpreters: usize,
    channel_size: usize,
    shutdown: CancellationToken,
}

impl EmbeddedRuntimeBuilder {
    pub fn new() -> Self {
        Self {
            frozen: Vec::new(),
            native: Vec::new(),
            num_interpreters: 4,
            channel_size: 32,
            shutdown: CancellationToken::new(),
        }
    }

    /// Add a frozen module set (e.g. from `py_freeze!(dir = "...")`)
    pub fn frozen<I>(mut self, lib: I) -> Self
    where
        I: IntoIterator<Item = (&'static str, FrozenModule)>,
    {
        self.frozen.extend(lib);
        self
    }

    /// Add a native (Rust) module into the interpreter.
    pub fn native(mut self, factory: NativeModuleDef) -> Self {
        self.native.push(factory);
        self
    }

    /// Add multiple native (Rust) modules into the interpreter.
    pub fn natives<I: IntoIterator<Item = NativeModuleDef>>(mut self, factories: I) -> Self {
        self.native.extend(factories);
        self
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

    /// Build the [`EmbeddedRuntime`], spawning worker threads and initializing interpreters.
    pub fn build(self) -> Result<EmbeddedRuntime, RuntimeError> {
        EmbeddedRuntime::new(
            self.frozen,
            self.native,
            self.num_interpreters,
            self.channel_size,
            self.shutdown,
        )
    }
}

impl Default for EmbeddedRuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::python::runtime::traits::RuntimeExt;
    use serde_json::json;

    fn runtime() -> EmbeddedRuntime {
        EmbeddedRuntime::builder()
            .num_interpreters(1)
            .channel_size(32)
            .build()
            .expect("failed to create EmbeddedRuntime")
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
