// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use guestjs::{guest::Guest, handle::module::Module, runtime::Runtime};

use crate::backend::ExecutorBackend;

/// The worker-local GuestJS environment supplied to an execution.
pub struct Context<B: ExecutorBackend> {
    runtime: Runtime<B>,
    guest: Guest<B>,
    module: Module<B>,
}

impl<B: ExecutorBackend> Context<B> {
    pub(crate) fn new(runtime: Runtime<B>, guest: Guest<B>, module: Module<B>) -> Self {
        Self { runtime, guest, module }
    }

    async fn close(
        runtime: Runtime<B>,
        guest: Guest<B>,
        module: Option<Module<B>>,
    ) -> Result<(), guestjs::errors::Error> {
        drop(module);

        guest.close();

        drop(guest);
        drop(runtime);

        Ok(())
    }

    pub(crate) async fn close_initialization(
        runtime: Runtime<B>,
        guest: Guest<B>,
    ) -> Result<(), guestjs::errors::Error> {
        Self::close(runtime, guest, None).await
    }

    pub(crate) async fn shutdown(self) -> Result<(), guestjs::errors::Error> {
        let Self { runtime, guest, module } = self;

        Self::close(runtime, guest, Some(module)).await
    }

    /// Returns the GuestJS runtime that owns this context.
    pub fn runtime(&self) -> &Runtime<B> {
        &self.runtime
    }

    /// Returns the persistent GuestJS guest for this worker.
    pub fn guest(&self) -> &Guest<B> {
        &self.guest
    }

    /// Returns the evaluated package module for this worker.
    pub fn module(&self) -> &Module<B> {
        &self.module
    }
}
