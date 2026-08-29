// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use guestpy::{guest::Guest, handle::Module, runtime::Runtime};

use crate::backend::ExecutorBackend;

pub struct Context<B: ExecutorBackend> {
    runtime: Runtime<B>,
    guest: Guest<B>,
    module: Module<B>,
}

impl<B: ExecutorBackend> Context<B> {
    pub(crate) fn new(runtime: Runtime<B>, guest: Guest<B>, module: Module<B>) -> Self {
        Self {
            runtime,
            guest,
            module,
        }
    }

    async fn close(
        runtime: Runtime<B>,
        guest: Guest<B>,
        module: Option<Module<B>>,
    ) -> Result<(), guestpy::errors::Error> {
        let mut failure = None;

        if let Some(driver) = guest.async_driver()
            && let Err(error) = driver.close().await
        {
            failure = Some(error);
        }

        drop(module);

        if let Err(error) = guest.close()
            && failure.is_none()
        {
            failure = Some(error);
        }

        drop(guest);

        if let Err(error) = runtime.shutdown()
            && failure.is_none()
        {
            failure = Some(error);
        }

        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    pub(crate) async fn close_initialization(
        runtime: Runtime<B>,
        guest: Guest<B>,
    ) -> Result<(), guestpy::errors::Error> {
        Self::close(runtime, guest, None).await
    }

    pub(crate) async fn shutdown(self) -> Result<(), guestpy::errors::Error> {
        let Self {
            runtime,
            guest,
            module,
        } = self;

        Self::close(runtime, guest, Some(module)).await
    }

    pub fn runtime(&self) -> &Runtime<B> {
        &self.runtime
    }

    pub fn guest(&self) -> &Guest<B> {
        &self.guest
    }

    pub fn module(&self) -> &Module<B> {
        &self.module
    }
}
