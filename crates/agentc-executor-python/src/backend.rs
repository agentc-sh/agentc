// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use guestpy::backend::{
    Backend, BackendCallables, BackendCoroutines, BackendExceptions, BackendInterrupt,
    BackendModules, BackendValues,
};

pub trait ExecutorBackend:
    Backend
    + BackendValues
    + BackendCallables
    + BackendModules
    + BackendCoroutines
    + BackendExceptions
    + BackendInterrupt
{
}

impl<B> ExecutorBackend for B where
    B: Backend
        + BackendValues
        + BackendCallables
        + BackendModules
        + BackendCoroutines
        + BackendExceptions
        + BackendInterrupt
{
}

#[cfg(test)]
mod tests {
    use guestpy::{pyo3::CPython, rustpython::RustPython};

    use crate::backend::ExecutorBackend;

    struct BackendContract;

    impl BackendContract {
        fn assert_backend<B: ExecutorBackend>() {}
    }

    #[test]
    fn supported_backends_satisfy_executor_contract() {
        BackendContract::assert_backend::<RustPython>();
        BackendContract::assert_backend::<CPython>();
    }
}
