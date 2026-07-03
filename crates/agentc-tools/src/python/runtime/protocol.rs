// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::de::DeserializeOwned;
use serde_json::{Value, from_value};
use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::oneshot;

use crate::python::runtime::errors::RuntimeError;

/// A native Rust callable that can be exposed as a Python function.
///
/// Receives the Python call arguments as a [`FunctionArgs`] and returns a JSON value,
/// or a [`RuntimeError`] if the call fails. The callable is reference-counted so it
/// can be shared across worker threads and cloned into commands without cost.
pub type NativeCallable =
    Arc<dyn Fn(FunctionArgs) -> Result<Value, RuntimeError> + Send + Sync + 'static>;

/// A single argument value passed to a Python function: either a JSON-serializable
/// value or a native Rust callable exposed as a Python function.
#[derive(Clone)]
pub enum ArgValue {
    Json(Value),
    Callable(NativeCallable),
}

impl Debug for ArgValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            ArgValue::Json(v) => write!(f, "Json({v:?})"),
            ArgValue::Callable(_) => write!(f, "Callable(...)"),
        }
    }
}

impl From<Value> for ArgValue {
    fn from(v: Value) -> Self {
        ArgValue::Json(v)
    }
}

impl From<NativeCallable> for ArgValue {
    fn from(f: NativeCallable) -> Self {
        ArgValue::Callable(f)
    }
}

/// Arguments to a Python function call. Supports positional and keyword args where
/// each value is either a JSON-serializable value or a native Rust callable.
#[derive(Debug, Clone, Default)]
pub struct FunctionArgs {
    pub positional: Vec<ArgValue>,
    pub keyword: Vec<(String, ArgValue)>,
}

impl FunctionArgs {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a positional argument.
    pub fn positional(mut self, arg: impl Into<ArgValue>) -> Self {
        self.positional.push(arg.into());
        self
    }

    pub fn positional_callable<F>(mut self, arg: F) -> Self
    where
        F: Fn(FunctionArgs) -> Result<Value, RuntimeError> + Send + Sync + 'static,
    {
        self.positional
            .push(ArgValue::Callable(Arc::new(arg)));
        self
    }

    /// Append a keyword argument.
    pub fn keyword(mut self, name: impl Into<String>, arg: impl Into<ArgValue>) -> Self {
        self.keyword
            .push((name.into(), arg.into()));
        self
    }

    pub fn keyword_callable<F>(mut self, name: impl Into<String>, arg: F) -> Self
    where
        F: Fn(FunctionArgs) -> Result<Value, RuntimeError> + Send + Sync + 'static,
    {
        self.keyword
            .push((name.into(), ArgValue::Callable(Arc::new(arg))));
        self
    }
}

impl From<Value> for FunctionArgs {
    fn from(v: Value) -> Self {
        FunctionArgs::new().positional(v)
    }
}

/// A future resolving to a deserialized Python result.
///
/// Returned by all [`PythonRuntime`] methods. Polls a [`oneshot::Receiver`] carrying a
/// raw [`Value`] and deserializes it into `T` on completion. Using a concrete type
/// parameter here means callers like `runtime.eval::<i32>(...)` get the deserialization
/// for free without any intermediate allocation.
pub struct PyFuture<T> {
    rx: oneshot::Receiver<Result<Value, RuntimeError>>,
    _marker: PhantomData<fn() -> T>,
}

impl<T> PyFuture<T> {
    pub fn new(rx: oneshot::Receiver<Result<Value, RuntimeError>>) -> Self {
        Self { rx, _marker: PhantomData }
    }
}

impl<T: DeserializeOwned> Future for PyFuture<T> {
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

/// Commands that can be dispatched to a Python interpreter.
///
/// Each variant is backend-agnostic. The backend translates each command into the
/// appropriate interpreter calls and returns a raw JSON [`Value`].
#[derive(Debug, Clone)]
pub enum Command {
    /// Evaluate a Python expression and return the JSON-serialized result.
    Eval { source: String },
    /// Exec a Python statement and return None.
    Exec { source: String },
    /// Set a global variable in the interpreter.
    SetGlobal { name: String, value: Value },
    /// Get a global variable from the interpreter.
    GetGlobal { name: String },
    /// List all global variables in the interpreter.
    ListGlobals,
    /// Import a module and return its top-level attribute names as a JSON array.
    Import { name: String },
    /// Call a named function from a module with the provided args.
    CallFunction {
        module: String,
        name: String,
        args: FunctionArgs,
    },
    /// Instantiate a named class from a module and call a method on the instance.
    CallMethod {
        module: String,
        class_name: String,
        method: String,
        args: FunctionArgs,
    },
}
