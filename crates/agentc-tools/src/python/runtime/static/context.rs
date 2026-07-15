// SPDX-FileCopyrightText: 2026 Timothy Pogue
//
// SPDX-License-Identifier: LicenseRef-Proprietary

use pyo3::{
    exceptions::PyRuntimeError,
    prelude::*,
    types::{PyCFunction, PyDict, PyTuple},
};
use pythonize::{depythonize, pythonize};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, to_value};
use std::ffi::CString;

use crate::python::runtime::{
    errors::RuntimeError,
    protocol::{ArgValue, Command, FunctionArgs},
};

/// The interpreter context that lives on the worker thread.
///
/// Owns the persistent globals namespace as a GIL-independent [`Py<PyDict>`] so it can
/// be re-bound under a fresh [`Python::attach`] for each command. Attaching per command
/// rather than holding the GIL across the loop lets other workers in the pool make
/// progress, since CPython serializes all threads behind a single process-wide GIL.
pub(super) struct InterpreterContext {
    globals: Py<PyDict>,
}

impl InterpreterContext {
    pub(super) fn new(py: Python<'_>) -> Self {
        Self { globals: PyDict::new(py).unbind() }
    }

    pub(super) fn dispatch(&self, py: Python<'_>, command: Command) -> Result<Value, RuntimeError> {
        match command {
            Command::Eval { source } => Self::deserialize_py(&py.eval(
                Self::to_cstr(&source)?.as_c_str(),
                Some(self.globals.bind(py)),
                None,
            )?),
            Command::Exec { source } => {
                py.run(Self::to_cstr(&source)?.as_c_str(), Some(self.globals.bind(py)), None)?;

                Ok(Value::Null)
            }
            Command::SetGlobal { name, value } => {
                self.globals
                    .bind(py)
                    .set_item(name, Self::serialize_py(py, &value)?)?;

                Ok(Value::Null)
            }
            Command::GetGlobal { name } => Self::deserialize_py(
                &self
                    .globals
                    .bind(py)
                    .get_item(&name)?
                    .ok_or_else(|| {
                        RuntimeError::python(format!("global '{name}' is not defined"))
                    })?,
            ),
            Command::ListGlobals => to_value(
                self.globals
                    .bind(py)
                    .keys()
                    .iter()
                    .filter_map(|k| k.extract::<String>().ok())
                    .collect::<Vec<_>>(),
            )
            .map_err(RuntimeError::serialize),
            Command::Import { name } => {
                let module = py.import(name.as_str())?;

                let attributes = module
                    .dict()
                    .keys()
                    .iter()
                    .filter_map(|k| k.extract::<String>().ok())
                    .collect::<Vec<_>>();

                self.globals
                    .bind(py)
                    .set_item(name.as_str(), &module)?;

                to_value(attributes).map_err(RuntimeError::serialize)
            }
            Command::CallFunction { module, name, args } => {
                let func = self
                    .resolve_module(py, &module)?
                    .getattr(name.as_str())?;

                let (positional, keyword) = Self::args_to_py(py, args)?;

                Self::deserialize_py(&func.call(positional, keyword.as_ref())?)
            }
            Command::CallMethod { module, class_name, method, args } => {
                let instance = self
                    .resolve_module(py, &module)?
                    .getattr(class_name.as_str())?
                    .call0()?;

                let (positional, keyword) = Self::args_to_py(py, args)?;

                Self::deserialize_py(&instance.call_method(
                    method.as_str(),
                    positional,
                    keyword.as_ref(),
                )?)
            }
        }
    }

    /// Resolve a module reference to a Python object, preferring a matching global and
    /// falling back to importing it by name.
    fn resolve_module<'py>(
        &self,
        py: Python<'py>,
        module: &str,
    ) -> Result<Bound<'py, PyAny>, RuntimeError> {
        match self.globals.bind(py).get_item(module)? {
            Some(obj) => Ok(obj),
            None => Ok(py.import(module)?.into_any()),
        }
    }

    /// Convert Python source into a NUL-terminated [`CString`] for the CPython C API.
    fn to_cstr(source: &str) -> Result<CString, RuntimeError> {
        CString::new(source).map_err(|e| RuntimeError::python(e.to_string()))
    }

    /// Convert a [`Value`] into a Python object via `pythonize`.
    fn serialize_py<'py, T: Serialize>(
        py: Python<'py>,
        value: &T,
    ) -> Result<Bound<'py, PyAny>, RuntimeError> {
        pythonize(py, value).map_err(|e| RuntimeError::python(e.to_string()))
    }

    /// Convert a Python object into a `T` via `depythonize`.
    fn deserialize_py<T: DeserializeOwned>(obj: &Bound<'_, PyAny>) -> Result<T, RuntimeError> {
        depythonize(obj).map_err(|e| RuntimeError::python(e.to_string()))
    }

    /// Convert a [`FunctionArgs`] into a positional [`PyTuple`] and an optional keyword [`PyDict`].
    fn args_to_py<'py>(
        py: Python<'py>,
        args: FunctionArgs,
    ) -> Result<(Bound<'py, PyTuple>, Option<Bound<'py, PyDict>>), RuntimeError> {
        let positional = PyTuple::new(
            py,
            args.positional
                .into_iter()
                .map(|arg| Self::arg_to_py(py, arg))
                .collect::<Result<Vec<_>, _>>()?,
        )?;

        let keyword = if args.keyword.is_empty() {
            None
        } else {
            let dict = PyDict::new(py);

            for (name, arg) in args.keyword {
                dict.set_item(name, Self::arg_to_py(py, arg)?)?;
            }

            Some(dict)
        };

        Ok((positional, keyword))
    }

    /// Convert a single [`ArgValue`] into a Python object.
    ///
    /// A [`NativeCallable`] is exposed as a closure-backed Python function so tool
    /// callbacks (e.g. `emit`) can be invoked from Python and marshalled back through
    /// the same JSON boundary.
    fn arg_to_py<'py>(py: Python<'py>, arg: ArgValue) -> Result<Bound<'py, PyAny>, RuntimeError> {
        match arg {
            ArgValue::Json(value) => Self::serialize_py(py, &value),
            ArgValue::Callable(callable) => Ok(PyCFunction::new_closure(
                py,
                None,
                None,
                move |args: &Bound<'_, PyTuple>, kwargs: Option<&Bound<'_, PyDict>>| {
                    let py = args.py();

                    let positional = args
                        .iter()
                        .map(|obj| {
                            depythonize(&obj)
                                .map(ArgValue::Json)
                                .map_err(|e| PyRuntimeError::new_err(e.to_string()))
                        })
                        .collect::<PyResult<Vec<_>>>()?;

                    let keyword = match kwargs {
                        Some(dict) => dict
                            .iter()
                            .map(|(name, value)| {
                                Ok((
                                    name.extract::<String>()?,
                                    ArgValue::Json(
                                        depythonize(&value)
                                            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?,
                                    ),
                                ))
                            })
                            .collect::<PyResult<Vec<_>>>()?,
                        None => Vec::new(),
                    };

                    pythonize(
                        py,
                        &callable(FunctionArgs { positional, keyword })
                            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?,
                    )
                    .map(|obj| obj.unbind())
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))
                },
            )?
            .into_any()),
        }
    }
}
