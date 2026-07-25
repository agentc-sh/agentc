// SPDX-FileCopyrightText: 2026 Timothy Pogue
//
// SPDX-License-Identifier: LicenseRef-Proprietary

use pyo3::{PyErr, Python, types::PyTracebackMethods};

use crate::python::runtime::errors::RuntimeError;

impl From<PyErr> for RuntimeError {
    fn from(err: PyErr) -> Self {
        RuntimeError::python(Python::attach(|py| {
            let traceback = err
                .traceback(py)
                .and_then(|tb| tb.format().ok())
                .unwrap_or_default();

            format!("{traceback}{err}")
        }))
    }
}
