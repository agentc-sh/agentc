// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use guestjs::backend::{
    Backend, buffers::BackendBuffers, callables::BackendCallables, classes::BackendClasses,
    exceptions::BackendExceptions, gc::BackendGc, interrupt::BackendInterrupt,
    library::BackendLibrary, modules::BackendModules, promises::BackendPromises,
    values::BackendValues,
};

pub trait ExecutorBackend:
    Backend
    + BackendValues
    + BackendCallables
    + BackendClasses
    + BackendExceptions
    + BackendModules
    + BackendPromises
    + BackendBuffers
    + BackendInterrupt
    + BackendLibrary
    + BackendGc
{
}

impl<B> ExecutorBackend for B where
    B: Backend
        + BackendValues
        + BackendCallables
        + BackendClasses
        + BackendExceptions
        + BackendModules
        + BackendPromises
        + BackendBuffers
        + BackendInterrupt
        + BackendLibrary
        + BackendGc
{
}

#[cfg(test)]
mod tests {
    use guestjs::quickjs::QuickJs;

    use crate::backend::ExecutorBackend;

    fn assert_backend<B: ExecutorBackend>() {}

    #[test]
    fn supported_backends_satisfy_executor_contract() {
        assert_backend::<QuickJs>();
    }
}
