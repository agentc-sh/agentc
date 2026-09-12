// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use guestpy::backend::{
    Backend, BackendCallables, BackendClasses, BackendCoroutines, BackendExceptions,
    BackendInterrupt, BackendLibrary, BackendModules, BackendValues,
};

pub trait ExecutorBackend:
    Backend
    + BackendValues
    + BackendCallables
    + BackendClasses
    + BackendModules
    + BackendCoroutines
    + BackendExceptions
    + BackendInterrupt
    + BackendLibrary
{
}

impl<B> ExecutorBackend for B where
    B: Backend
        + BackendValues
        + BackendCallables
        + BackendClasses
        + BackendModules
        + BackendCoroutines
        + BackendExceptions
        + BackendInterrupt
        + BackendLibrary
{
}

#[cfg(test)]
mod tests {
    use guestpy::{pyo3::CPython, rustpython::RustPython};

    use crate::backend::ExecutorBackend;

    fn assert_backend<B: ExecutorBackend>() {}

    #[test]
    fn supported_backends_satisfy_executor_contract() {
        assert_backend::<RustPython>();
        assert_backend::<CPython>();
    }
}
