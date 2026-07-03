// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::{future::Future, pin::Pin};

use crate::cli::{context::Ctx, errors::CliError};

#[async_trait]
pub trait Cmd: Send + Sync {
    async fn update_ctx(&self, _ctx: &mut Ctx) -> Result<(), CliError> {
        Ok(())
    }

    async fn run(&self, _ctx: &mut Ctx) -> Result<(), CliError> {
        Ok(())
    }

    fn next_cmd(&self) -> Option<&dyn Cmd> {
        None
    }
}

impl<'ctx> dyn Cmd + 'ctx {
    pub fn walk_execute(
        &'ctx self,
        ctx: &'ctx mut Ctx,
    ) -> Pin<Box<dyn Future<Output = Result<(), CliError>> + Send + 'ctx>> {
        Box::pin(async move {
            self.update_ctx(ctx).await?;
            self.run(ctx).await?;

            if let Some(next) = self.next_cmd() {
                next.walk_execute(ctx).await?;
            }

            Ok(())
        })
    }
}
