// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::de::DeserializeOwned;
use serde_json::{Value as JsonValue, from_value};
use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::oneshot;

use crate::javascript::runtime::errors::RuntimeError;

/// A native Rust callable that can be exposed as a JavaScript function.
///
/// Receives the JavaScript call arguments as a flat [`Vec<ArgValue>`] and returns a JSON
/// value, or a [`RuntimeError`] if the call fails. The callable is reference-counted so
/// it can be shared across worker threads and cloned into commands without cost.
pub type NativeCallable =
    Arc<dyn Fn(Vec<ArgValue>) -> Result<JsonValue, RuntimeError> + Send + Sync + 'static>;

/// A single argument value passed to a JavaScript function: a plain JSON value, a native
/// Rust callable exposed as a JS function, or a JS object literal whose fields may
/// themselves be callables.
#[derive(Clone)]
pub enum ArgValue {
    Json(JsonValue),
    Callable(NativeCallable),
    /// A JS object literal. Fields are ordered (insertion order is preserved) and each
    /// value may recursively be any `ArgValue`, including another `Callable`.
    Object(Vec<(String, ArgValue)>),
}

impl Debug for ArgValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            ArgValue::Json(v) => write!(f, "Json({v:?})"),
            ArgValue::Callable(_) => write!(f, "Callable(...)"),
            ArgValue::Object(fields) => write!(f, "Object({fields:?})"),
        }
    }
}

impl From<JsonValue> for ArgValue {
    fn from(v: JsonValue) -> Self {
        ArgValue::Json(v)
    }
}

impl From<NativeCallable> for ArgValue {
    fn from(f: NativeCallable) -> Self {
        ArgValue::Callable(f)
    }
}

/// Arguments to a JavaScript function call. JavaScript has only positional parameters
/// (no keyword arguments), so `FunctionArgs` is a flat list of [`ArgValue`]s.
///
/// When calling a tool's `execute` method, the convention is to pass a single
/// `ArgValue::Object` param containing all named fields (e.g. `args`, `state`, `emit`).
#[derive(Debug, Clone, Default)]
pub struct FunctionArgs {
    pub params: Vec<ArgValue>,
}

impl FunctionArgs {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a positional parameter.
    pub fn param(mut self, arg: impl Into<ArgValue>) -> Self {
        self.params.push(arg.into());
        self
    }

    /// Append a native Rust callable as a positional parameter.
    pub fn param_callable<F>(mut self, f: F) -> Self
    where
        F: Fn(Vec<ArgValue>) -> Result<JsonValue, RuntimeError> + Send + Sync + 'static,
    {
        self.params
            .push(ArgValue::Callable(Arc::new(f)));
        self
    }
}

impl From<JsonValue> for FunctionArgs {
    fn from(v: JsonValue) -> Self {
        FunctionArgs::new().param(ArgValue::Json(v))
    }
}

/// Commands that can be dispatched to a QuickJS interpreter.
///
/// Each variant is backend-agnostic. The interpreter translates each command into the
/// appropriate QuickJS operations and returns a raw JSON [`JsonValue`].
#[derive(Clone)]
pub enum Command {
    /// Call the module's default export as an async function.
    CallDefault { args: FunctionArgs },
    /// Call a named exported function with args.
    CallFunction { name: String, args: FunctionArgs },
    /// Fetch a named export object and call its `execute` property with args.
    CallExportMethod { export: String, args: FunctionArgs },
    /// Retrieve a named export from the module.
    GetExport { name: String },
    /// Set a global variable on the context.
    SetGlobal { name: String, value: JsonValue },
    /// Retrieve a global variable from the context.
    GetGlobal { name: String },
    /// Evaluate a JS expression and return its result.
    Eval { source: String },
}

/// A future resolving to a deserialized QuickJS result.
///
/// Returned by all [`Runtime`](crate::javascript::runtime::traits::Runtime) methods. Polls a
/// [`oneshot::Receiver`] carrying a raw [`JsonValue`] and deserializes it into `T` on completion.
pub struct JsFuture<T> {
    rx: oneshot::Receiver<Result<JsonValue, RuntimeError>>,
    _marker: PhantomData<fn() -> T>,
}

impl<T> JsFuture<T> {
    pub fn new(rx: oneshot::Receiver<Result<JsonValue, RuntimeError>>) -> Self {
        Self { rx, _marker: PhantomData }
    }
}

impl<T: DeserializeOwned> Future for JsFuture<T> {
    type Output = Result<T, RuntimeError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.rx)
            .poll(cx)
            .map(|res| match res {
                Ok(Ok(value)) => from_value::<T>(value).map_err(RuntimeError::serialize),
                Ok(Err(err)) => Err(err),
                Err(_) => Err(RuntimeError::worker_closed()),
            })
    }
}
