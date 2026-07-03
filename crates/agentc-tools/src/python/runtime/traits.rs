// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, to_value};
use std::{sync::Arc, time::Duration};
use tokio::{sync::oneshot, time::timeout};

use crate::python::runtime::{
    errors::RuntimeError,
    protocol::{Command, FunctionArgs, PyFuture},
};

/// Trait implemented by each Python backend (embedded RustPython, static CPython via PyO3).
pub trait Runtime: Send + Sync + 'static {
    /// Dispatch a command to the runtime and return a receiver for the result.
    fn dispatch(&self, command: Command) -> oneshot::Receiver<Result<Value, RuntimeError>>;
}

/// Extension methods for Runtime to provide a more convenient API for common operations.
#[async_trait]
pub trait RuntimeExt: Runtime {
    /// Dispatch a command to the worker thread and return a future for the result.
    fn send<T>(&self, command: Command) -> PyFuture<T> {
        PyFuture::new(self.dispatch(command))
    }

    /// Evaluate a Python expression and return its deserialized result.
    async fn eval<T: DeserializeOwned>(&self, source: &str) -> Result<T, RuntimeError> {
        self.send(Command::Eval { source: source.to_string() })
            .await
    }

    /// Evaluate a Python expression and return its deserialized result with a timeout.
    async fn eval_with_timeout<T: DeserializeOwned>(
        &self,
        source: &str,
        duration: Duration,
    ) -> Result<T, RuntimeError> {
        timeout(duration, self.eval(source))
            .await
            .map_err(|_| RuntimeError::timeout())?
    }

    /// Exec a Python statement.
    async fn exec(&self, source: &str) -> Result<(), RuntimeError> {
        self.send(Command::Exec { source: source.to_string() })
            .await
    }

    /// Exec a Python statement with a timeout.
    async fn exec_with_timeout(
        &self,
        source: &str,
        duration: Duration,
    ) -> Result<(), RuntimeError> {
        timeout(duration, self.exec(source))
            .await
            .map_err(|_| RuntimeError::timeout())?
    }

    /// Set a global variable in the interpreter.
    async fn set_global<V>(&self, name: &str, value: V) -> Result<(), RuntimeError>
    where
        V: Serialize + Send,
    {
        self.send(Command::SetGlobal {
            name: name.to_string(),
            value: to_value(value).map_err(RuntimeError::serialize)?,
        })
        .await
    }

    /// Set a global variable in the interpreter with a timeout.
    async fn set_global_with_timeout<V>(
        &self,
        name: &str,
        value: V,
        duration: Duration,
    ) -> Result<(), RuntimeError>
    where
        V: Serialize + Send,
    {
        timeout(duration, self.set_global(name, value))
            .await
            .map_err(|_| RuntimeError::timeout())?
    }

    /// Get a global variable from the interpreter.
    async fn get_global<T: DeserializeOwned>(&self, name: &str) -> Result<T, RuntimeError> {
        self.send(Command::GetGlobal { name: name.to_string() })
            .await
    }

    /// Get a global variable from the interpreter with a timeout.
    async fn get_global_with_timeout<T: DeserializeOwned>(
        &self,
        name: &str,
        duration: Duration,
    ) -> Result<T, RuntimeError> {
        timeout(duration, self.get_global(name))
            .await
            .map_err(|_| RuntimeError::timeout())?
    }

    /// List all global variables in the interpreter.
    async fn list_globals(&self) -> Result<Vec<String>, RuntimeError> {
        self.send(Command::ListGlobals).await
    }

    /// List all global variables in the interpreter with a timeout.
    async fn list_globals_with_timeout(
        &self,
        duration: Duration,
    ) -> Result<Vec<String>, RuntimeError> {
        timeout(duration, self.list_globals())
            .await
            .map_err(|_| RuntimeError::timeout())?
    }

    /// Import a module into the interpreter and get its top level attribute names as a vector of strings.
    async fn import(&self, name: &str) -> Result<Vec<String>, RuntimeError> {
        self.send(Command::Import { name: name.to_string() })
            .await
    }

    /// Import a module into the interpreter with a timeout.
    async fn import_with_timeout(
        &self,
        name: &str,
        duration: Duration,
    ) -> Result<Vec<String>, RuntimeError> {
        timeout(duration, self.import(name))
            .await
            .map_err(|_| RuntimeError::timeout())?
    }

    /// Call a named function from a module with the provided args.
    async fn call_function<T: DeserializeOwned>(
        &self,
        module: &str,
        name: &str,
        args: FunctionArgs,
    ) -> Result<T, RuntimeError> {
        self.send(Command::CallFunction {
            module: module.to_string(),
            name: name.to_string(),
            args,
        })
        .await
    }

    /// Call a named function from a module with the provided args and a timeout.
    async fn call_function_with_timeout<T: DeserializeOwned>(
        &self,
        module: &str,
        name: &str,
        args: FunctionArgs,
        duration: Duration,
    ) -> Result<T, RuntimeError> {
        timeout(duration, self.call_function(module, name, args))
            .await
            .map_err(|_| RuntimeError::timeout())?
    }

    /// Instantiate a named class from a module and call a method on the instance.
    async fn call_method<T: DeserializeOwned>(
        &self,
        module: &str,
        class_name: &str,
        method: &str,
        args: FunctionArgs,
    ) -> Result<T, RuntimeError> {
        self.send(Command::CallMethod {
            module: module.to_string(),
            class_name: class_name.to_string(),
            method: method.to_string(),
            args,
        })
        .await
    }

    /// Instantiate a named class from a module and call a method on the instance with a timeout.
    async fn call_method_with_timeout<T: DeserializeOwned>(
        &self,
        module: &str,
        class_name: &str,
        method: &str,
        args: FunctionArgs,
        duration: Duration,
    ) -> Result<T, RuntimeError> {
        timeout(duration, self.call_method(module, class_name, method, args))
            .await
            .map_err(|_| RuntimeError::timeout())?
    }
}

impl<R: Runtime + ?Sized> RuntimeExt for R {}

impl Runtime for Arc<dyn Runtime> {
    fn dispatch(&self, command: Command) -> oneshot::Receiver<Result<Value, RuntimeError>> {
        (**self).dispatch(command)
    }
}
