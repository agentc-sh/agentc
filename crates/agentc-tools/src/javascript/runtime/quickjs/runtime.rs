// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
};

use rquickjs::{
    AsyncContext, AsyncRuntime, Function, Module, Object, Persistent, Value, async_with,
    loader::{BuiltinResolver, ModuleLoader},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value as JsonValue;
use tokio::{
    sync::{mpsc, oneshot},
    task::LocalSet,
};
use tokio_util::sync::CancellationToken;

use agentc_agent::types::capability::{Capability, CapabilitySet};

use crate::javascript::runtime::{
    errors::RuntimeError,
    protocol::{ArgValue, Command, FunctionArgs},
    traits::Runtime,
};

type InitHook = Arc<dyn Fn(&rquickjs::Ctx<'_>) -> Result<(), rquickjs::Error> + Send + Sync>;

struct InterpreterMessage {
    command: Command,
    tx: oneshot::Sender<Result<JsonValue, RuntimeError>>,
}

/// The QuickJS interpreter context that lives on the worker thread.
///
/// Owns the persistent module namespace and default function export. All command dispatch
/// and JS/Rust value conversion happens through methods on this struct, so the interpreter
/// context is never accessed outside the worker thread.
struct InterpreterContext {
    context: AsyncContext,
    namespace: Persistent<Object<'static>>,
    default_fn: Option<Persistent<Function<'static>>>,
}

impl InterpreterContext {
    fn extract_exception(ctx: &rquickjs::Ctx<'_>) -> String {
        let val = ctx.catch();
        let msg = val
            .as_object()
            .and_then(|o| o.get::<_, String>("message").ok())
            .unwrap_or_else(|| format!("{val:?}"));
        let stack = val
            .as_object()
            .and_then(|o| o.get::<_, String>("stack").ok())
            .unwrap_or_default();

        if stack.is_empty() {
            msg
        } else {
            format!("{msg}\n{stack}")
        }
    }

    async fn dispatch(&self, command: Command) -> Result<JsonValue, RuntimeError> {
        let ns = self.namespace.clone();
        let df = self.default_fn.clone();

        async_with!(self.context => |ctx| {
            match Self::dispatch_inner(&ctx, command, &ns, &df).await {
                Err(rquickjs::Error::Exception) => {
                    Err(RuntimeError::js_with_message(rquickjs::Error::Exception, Self::extract_exception(&ctx)))
                }
                result => result.map_err(RuntimeError::js),
            }
        })
        .await
    }

    async fn dispatch_inner<'js>(
        ctx: &rquickjs::Ctx<'js>,
        command: Command,
        namespace: &Persistent<Object<'static>>,
        default_fn: &Option<Persistent<Function<'static>>>,
    ) -> Result<JsonValue, rquickjs::Error> {
        match command {
            Command::CallDefault { args } => {
                Self::call_fn(
                    ctx,
                    default_fn
                        .as_ref()
                        .ok_or(rquickjs::Error::new_resolving("tool.js", "default"))?
                        .clone()
                        .restore(ctx)?,
                    args,
                )
                .await
            }

            Command::CallFunction { name, args } => {
                Self::call_fn(
                    ctx,
                    namespace
                        .clone()
                        .restore(ctx)?
                        .get(name.as_str())?,
                    args,
                )
                .await
            }

            Command::CallExportMethod { export, args } => {
                Self::call_fn(
                    ctx,
                    namespace
                        .clone()
                        .restore(ctx)?
                        .get::<_, Object>(export.as_str())?
                        .get("execute")?,
                    args,
                )
                .await
            }

            Command::GetExport { name } => Self::from_js(
                namespace
                    .clone()
                    .restore(ctx)?
                    .get(name.as_str())?,
            ),

            Command::SetGlobal { name, value } => {
                ctx.globals()
                    .set(name.as_str(), Self::to_js(ctx, &value)?)?;
                Ok(JsonValue::Null)
            }

            Command::GetGlobal { name } => Self::from_js(ctx.globals().get(name.as_str())?),

            Command::Eval { source } => Self::from_js(ctx.eval(source.as_bytes())?),
        }
    }

    async fn call_fn<'js>(
        ctx: &rquickjs::Ctx<'js>,
        func: Function<'js>,
        args: FunctionArgs,
    ) -> Result<JsonValue, rquickjs::Error> {
        let mut call_args = rquickjs::function::Args::new_unsized(ctx.clone());

        for arg in args.params {
            call_args.push_arg(Self::arg_to_js(ctx, arg)?)?;
        }

        let ret = func.call_arg::<Value<'js>>(call_args)?;

        Self::from_js(if ret.is_promise() {
            ret.into_promise()
                .ok_or(rquickjs::Error::Exception)?
                .into_future::<Value>()
                .await?
        } else {
            ret
        })
    }

    fn arg_to_js<'js>(
        ctx: &rquickjs::Ctx<'js>,
        arg: ArgValue,
    ) -> Result<Value<'js>, rquickjs::Error> {
        match arg {
            ArgValue::Json(v) => Self::to_js(ctx, &v),
            ArgValue::Callable(callable) => Ok(Function::new(
                ctx.clone(),
                move |ctx: rquickjs::Ctx<'js>,
                      args: rquickjs::function::Rest<Value<'js>>|
                      -> rquickjs::Result<Value<'js>> {
                    Self::to_js(
                        &ctx,
                        &callable(
                            args.0
                                .into_iter()
                                .map(|v| Self::from_js(v).map(ArgValue::Json))
                                .collect::<Result<Vec<_>, _>>()
                                .map_err(|_| rquickjs::Error::Exception)?,
                        )
                        .map_err(|_| rquickjs::Error::Exception)?,
                    )
                },
            )?
            .into_value()),
            ArgValue::Object(fields) => {
                let obj = Object::new(ctx.clone())?;
                for (key, val) in fields {
                    obj.set(key.as_str(), Self::arg_to_js(ctx, val)?)?;
                }
                Ok(obj.into_value())
            }
        }
    }

    fn to_js<'js, T: Serialize>(
        ctx: &rquickjs::Ctx<'js>,
        val: &T,
    ) -> Result<Value<'js>, rquickjs::Error> {
        rquickjs_serde::to_value(ctx.clone(), val).map_err(|_| rquickjs::Error::Exception)
    }

    fn from_js<'js, T: DeserializeOwned>(val: Value<'js>) -> Result<T, rquickjs::Error> {
        rquickjs_serde::from_value(val).map_err(|_| rquickjs::Error::Exception)
    }
}

/// The spawner/runner for a single QuickJS worker thread.
///
/// Initializes the rquickjs runtime and context, evaluates the bundled JS module, and
/// produces an [`InterpreterContext`] that the run loop uses to dispatch commands.
struct QuickJsInterpreter {
    source: String,
    capabilities: CapabilitySet,
    init_hook: Option<InitHook>,
    shutdown: CancellationToken,
}

impl QuickJsInterpreter {
    fn new(
        source: String,
        capabilities: CapabilitySet,
        init_hook: Option<InitHook>,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            source,
            capabilities,
            init_hook,
            shutdown,
        }
    }

    async fn spawn(
        self,
    ) -> Result<(mpsc::Sender<InterpreterMessage>, JoinHandle<()>), RuntimeError> {
        let (ready_tx, ready_rx) =
            oneshot::channel::<Result<mpsc::Sender<InterpreterMessage>, RuntimeError>>();

        let handle = thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build worker tokio runtime");

            LocalSet::new().block_on(&rt, async move {
                self.run(ready_tx).await;
            });
        });

        let tx = ready_rx
            .await
            .map_err(|_| RuntimeError::WorkerClosed)??;

        Ok((tx, handle))
    }

    async fn run(
        self,
        ready: oneshot::Sender<Result<mpsc::Sender<InterpreterMessage>, RuntimeError>>,
    ) {
        let runtime = match AsyncRuntime::new() {
            Ok(r) => r,
            Err(e) => {
                let _ = ready.send(Err(RuntimeError::init(e)));
                return;
            }
        };

        runtime
            .set_max_stack_size(512 * 1024)
            .await;
        Self::init_module_loaders(&self.capabilities, &runtime).await;

        let context = match AsyncContext::full(&runtime).await {
            Ok(c) => c,
            Err(e) => {
                let _ = ready.send(Err(RuntimeError::init(e)));
                return;
            }
        };

        let capabilities = self.capabilities.clone();
        let source = self.source.clone();
        let init_hook = self.init_hook.clone();

        let init_result = async_with!(context => |ctx| {
            Self::init_globals(&capabilities, &ctx, init_hook)?;
            Self::init_process_env(&ctx)?;

            let module = Module::declare(ctx.clone(), "tool.js", source.as_bytes())?;
            let (module, eval_promise) = module.eval()?;
            eval_promise.into_future::<Value>().await?;

            let namespace = module.namespace()?;
            let persistent_namespace = Persistent::save(&ctx, namespace);

            let persistent_default = persistent_namespace
                .clone()
                .restore(&ctx)?
                .get::<_, Value<'_>>("default")
                .ok()
                .filter(|v| v.is_function())
                .map(|v| Persistent::save(&ctx, Function::from_value(v).unwrap()));

            Ok::<_, rquickjs::Error>((persistent_namespace, persistent_default))
        })
        .await;

        let (persistent_namespace, persistent_default) = match init_result {
            Ok(p) => p,
            Err(e) => {
                let _ = ready.send(Err(RuntimeError::module_load(e)));
                return;
            }
        };

        let interp_ctx = InterpreterContext {
            context,
            namespace: persistent_namespace,
            default_fn: persistent_default,
        };

        let (tx, mut rx) = mpsc::channel::<InterpreterMessage>(32);
        let _ = ready.send(Ok(tx));

        loop {
            let msg = tokio::select! {
                msg = rx.recv() => msg,
                _ = self.shutdown.cancelled() => break,
            };

            let Some(InterpreterMessage { command, tx: reply }) = msg else {
                break;
            };
            let _ = reply.send(interp_ctx.dispatch(command).await);
        }
    }

    async fn init_module_loaders(capabilities: &CapabilitySet, runtime: &AsyncRuntime) {
        let mut resolver = BuiltinResolver::default();
        let mut loader = ModuleLoader::default();

        if capabilities.has_any(&[
            Capability::from("filesystem::read"),
            Capability::from("filesystem::write"),
        ]) {
            for name in &["node:fs", "fs"] {
                resolver.add_module(*name);
                loader.add_module(*name, llrt_modules::fs::FsModule);
            }
            for name in &["node:fs/promises", "fs/promises"] {
                resolver.add_module(*name);
                loader.add_module(*name, llrt_modules::fs::FsPromisesModule);
            }
        }

        for name in &["node:os", "os"] {
            resolver.add_module(*name);
            loader.add_module(*name, llrt_modules::os::OsModule);
        }

        runtime
            .set_loader(resolver, loader)
            .await;
    }

    fn init_globals(
        capabilities: &CapabilitySet,
        ctx: &rquickjs::Ctx<'_>,
        init_hook: Option<InitHook>,
    ) -> Result<(), rquickjs::Error> {
        llrt_modules::console::init(ctx)?;
        llrt_modules::timers::init(ctx)?;
        llrt_modules::url::init(ctx)?;
        llrt_modules::buffer::init(ctx)?;

        if capabilities.has(&Capability::from("network")) {
            llrt_modules::fetch::init(ctx)?;
        }

        if let Some(hook) = init_hook {
            hook(ctx)?;
        }

        Ok(())
    }

    fn init_process_env(ctx: &rquickjs::Ctx<'_>) -> Result<(), rquickjs::Error> {
        let globals = ctx.globals();

        let process: Object = match globals.get("process") {
            Ok(p) => p,
            Err(_) => {
                let p = Object::new(ctx.clone())?;
                globals.set("process", p.clone())?;
                p
            }
        };

        let env = Object::new(ctx.clone())?;
        for (key, value) in std::env::vars() {
            env.set(key, value)?;
        }
        process.set("env", env)?;

        Ok(())
    }
}

pub struct QuickJsRuntimeWorker {
    msg_tx: mpsc::Sender<InterpreterMessage>,
    handle: JoinHandle<()>,
}

impl QuickJsRuntimeWorker {
    pub async fn new(
        source: String,
        capabilities: CapabilitySet,
        init_hook: Option<InitHook>,
        shutdown: CancellationToken,
    ) -> Result<Self, RuntimeError> {
        let (tx, handle) = QuickJsInterpreter::new(source, capabilities, init_hook, shutdown)
            .spawn()
            .await?;
        Ok(Self { msg_tx: tx, handle })
    }

    pub fn close(self) -> Result<(), RuntimeError> {
        drop(self.msg_tx);

        self.handle.join().map_err(|err| {
            err.downcast::<RuntimeError>()
                .map(|e| *e)
                .unwrap_or(RuntimeError::WorkerClosed)
        })
    }

    fn send_command(&self, command: Command) -> oneshot::Receiver<Result<JsonValue, RuntimeError>> {
        let (reply_tx, reply_rx) = oneshot::channel();

        if self
            .msg_tx
            .try_send(InterpreterMessage { command, tx: reply_tx })
            .is_err()
        {
            let (err_tx, err_rx) = oneshot::channel();
            let _ = err_tx.send(Err(RuntimeError::WorkerClosed));
            return err_rx;
        }

        reply_rx
    }
}

/// A pool of QuickJS interpreter workers backed by a single bundled JS module.
///
/// Multiple [`JavascriptTool`](crate::javascript::tool::JavascriptTool) instances may share one
/// `QuickJsRuntime` via [`Arc`], each calling a different named export from the same module.
pub struct QuickJsRuntime {
    workers: Vec<QuickJsRuntimeWorker>,
    next: AtomicUsize,
}

impl QuickJsRuntime {
    pub fn builder() -> QuickJsRuntimeBuilder {
        QuickJsRuntimeBuilder::new()
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
            Err(RuntimeError::WorkerClosed)
        }
    }

    fn worker(&self) -> &QuickJsRuntimeWorker {
        &self.workers[self
            .next
            .fetch_add(1, Ordering::Relaxed)
            % self.workers.len()]
    }
}

impl Runtime for QuickJsRuntime {
    fn dispatch(&self, command: Command) -> oneshot::Receiver<Result<JsonValue, RuntimeError>> {
        self.worker().send_command(command)
    }
}

pub struct QuickJsRuntimeBuilder {
    source: String,
    capabilities: CapabilitySet,
    num_interpreters: usize,
    init_hook: Option<InitHook>,
    shutdown: CancellationToken,
}

impl QuickJsRuntimeBuilder {
    /// Create a new builder with default configuration. The default runtime has one worker, no capabilities, and no JS source.
    pub fn new() -> Self {
        Self {
            source: String::new(),
            capabilities: CapabilitySet::default(),
            num_interpreters: 1,
            init_hook: None,
            shutdown: CancellationToken::new(),
        }
    }

    /// Set the source code for the JS module to load in each worker. Must export at least one function.
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    /// Set the capabilities to expose to each worker. This controls which built-in modules are available
    /// and is also passed to the init hook for custom global configuration.
    pub fn capabilities<I, C>(mut self, capabilities: I) -> Self
    where
        I: IntoIterator<Item = C>,
        C: Into<Capability>,
    {
        self.capabilities.extend(capabilities);
        self
    }

    /// Set the number of worker threads in the runtime. Each worker has its own QuickJS context and executes JS calls independently.
    pub fn num_interpreters(mut self, n: usize) -> Self {
        self.num_interpreters = n;
        self
    }

    /// Register a hook called once per worker after globals are initialised.
    pub fn with_init_hook<F>(mut self, hook: F) -> Self
    where
        F: Fn(&rquickjs::Ctx<'_>) -> Result<(), rquickjs::Error> + Send + Sync + 'static,
    {
        self.init_hook = Some(Arc::new(hook));
        self
    }

    /// Set the shutdown token for all worker threads in this runtime.
    pub fn shutdown(mut self, token: CancellationToken) -> Self {
        self.shutdown = token;
        self
    }

    /// Build the runtime and spawn all worker threads. This is a fallible operation since it involves starting
    /// threads and initializing QuickJS contexts.
    pub async fn build(self) -> Result<QuickJsRuntime, RuntimeError> {
        let mut workers = Vec::with_capacity(self.num_interpreters);

        for _ in 0..self.num_interpreters {
            workers.push(
                QuickJsRuntimeWorker::new(
                    self.source.clone(),
                    self.capabilities.clone(),
                    self.init_hook.clone(),
                    self.shutdown.clone(),
                )
                .await?,
            );
        }

        Ok(QuickJsRuntime { workers, next: AtomicUsize::new(0) })
    }
}

impl Default for QuickJsRuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    use crate::javascript::runtime::{
        protocol::{ArgValue, FunctionArgs},
        traits::RuntimeExt,
    };

    const ECHO_JS: &str = r#"
        export async function echo(args) { return args; }
    "#;

    async fn plain_runtime(source: &str) -> QuickJsRuntime {
        QuickJsRuntimeBuilder::new()
            .source(source)
            .num_interpreters(1)
            .build()
            .await
            .expect("runtime failed to start")
    }

    #[tokio::test]
    async fn call_with_json_arg() {
        let runtime = plain_runtime(ECHO_JS).await;

        let result = runtime
            .call::<JsonValue>("echo", FunctionArgs::from(json!({"hello": "world"})))
            .await
            .expect("call failed");

        assert_eq!(result, json!({"hello": "world"}));
    }

    const CALLABLE_JS: &str = r#"
        export async function withCallback(input) {
            const result = input.fn(42);
            return { value: result };
        }
    "#;

    #[tokio::test]
    async fn callable_in_object_arg() {
        let runtime = plain_runtime(CALLABLE_JS).await;

        #[derive(serde::Deserialize)]
        struct Out {
            value: i32,
        }

        let fields = vec![(
            "fn".to_string(),
            ArgValue::Callable(Arc::new(|params| {
                let n = match params.first() {
                    Some(ArgValue::Json(JsonValue::Number(n))) => n.as_i64().unwrap_or(0) as i32,
                    _ => 0,
                };
                Ok(json!(n * 2))
            })),
        )];

        let result = runtime
            .call::<Out>("withCallback", FunctionArgs::new().param(ArgValue::Object(fields)))
            .await
            .expect("call failed");

        assert_eq!(result.value, 84);
    }

    const READ_GLOBAL_JS: &str = r#"
        export async function readMagic(args) { return { value: __magic__ }; }
    "#;

    #[tokio::test]
    async fn init_hook_registers_custom_global() {
        let runtime = QuickJsRuntimeBuilder::new()
            .source(READ_GLOBAL_JS)
            .num_interpreters(1)
            .with_init_hook(|ctx| {
                ctx.globals().set("__magic__", 42_i32)?;
                Ok(())
            })
            .build()
            .await
            .expect("runtime failed to start");

        #[derive(serde::Deserialize)]
        struct Out {
            value: i32,
        }

        let result = runtime
            .call::<Out>("readMagic", FunctionArgs::from(json!({})))
            .await
            .expect("call failed");

        assert_eq!(result.value, 42);
    }
}
