// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use guestjs::{
    handle::Module,
    runtime::{Guest, Runtime},
};

/// The worker-local GuestJS environment supplied to an execution.
pub struct Context {
    runtime: Runtime,
    guest: Guest,
    module: Module,
}

impl Context {
    pub(crate) fn new(runtime: Runtime, guest: Guest, module: Module) -> Self {
        Self { runtime, guest, module }
    }

    /// Returns the GuestJS runtime that owns this context.
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    /// Returns the persistent GuestJS guest for this worker.
    pub fn guest(&self) -> &Guest {
        &self.guest
    }

    /// Returns the evaluated package module for this worker.
    pub fn module(&self) -> &Module {
        &self.module
    }
}
