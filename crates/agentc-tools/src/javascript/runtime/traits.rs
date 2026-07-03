// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::de::DeserializeOwned;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tokio::sync::oneshot;

use crate::javascript::runtime::{
    errors::RuntimeError,
    protocol::{Command, FunctionArgs, JsFuture},
};

/// Trait implemented by each JavaScript backend (e.g. QuickJS).
pub trait Runtime: Send + Sync + 'static {
    /// Dispatch a command to the runtime and return a receiver for the result.
    fn dispatch(&self, command: Command) -> oneshot::Receiver<Result<JsonValue, RuntimeError>>;
}

/// Extension methods for [`Runtime`] that provide a typed, convenient API.
pub trait RuntimeExt: Runtime {
    /// Dispatch a command and return a future that deserializes the result into `T`.
    fn send<T: DeserializeOwned>(&self, command: Command) -> JsFuture<T> {
        JsFuture::new(self.dispatch(command))
    }

    /// Call the module's default export with `args`.
    fn call_default<T: DeserializeOwned>(&self, args: FunctionArgs) -> JsFuture<T> {
        self.send(Command::CallDefault { args })
    }

    /// Call a named exported function with `args`.
    fn call<T: DeserializeOwned>(
        &self,
        name: impl Into<String>,
        args: FunctionArgs,
    ) -> JsFuture<T> {
        self.send(Command::CallFunction { name: name.into(), args })
    }

    /// Fetch a named export object and call its `execute` property with `args`.
    fn call_export_method<T: DeserializeOwned>(
        &self,
        export: impl Into<String>,
        args: FunctionArgs,
    ) -> JsFuture<T> {
        self.send(Command::CallExportMethod { export: export.into(), args })
    }

    /// Retrieve a named export from the module.
    fn get_export<T: DeserializeOwned>(&self, name: impl Into<String>) -> JsFuture<T> {
        self.send(Command::GetExport { name: name.into() })
    }

    /// Set a global variable on the context.
    fn set_global<T: DeserializeOwned>(
        &self,
        name: impl Into<String>,
        value: JsonValue,
    ) -> JsFuture<T> {
        self.send(Command::SetGlobal { name: name.into(), value })
    }

    /// Retrieve a global variable from the context.
    fn get_global<T: DeserializeOwned>(&self, name: impl Into<String>) -> JsFuture<T> {
        self.send(Command::GetGlobal { name: name.into() })
    }

    /// Evaluate a JS expression and return its result.
    fn eval<T: DeserializeOwned>(&self, source: impl Into<String>) -> JsFuture<T> {
        self.send(Command::Eval { source: source.into() })
    }
}

impl<R: Runtime + ?Sized> RuntimeExt for R {}

impl Runtime for Arc<dyn Runtime> {
    fn dispatch(&self, command: Command) -> oneshot::Receiver<Result<JsonValue, RuntimeError>> {
        (**self).dispatch(command)
    }
}
