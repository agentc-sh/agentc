// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::{future::Future, pin::Pin};

use crate::cli::{context::Ctx, errors::CliError, types::CmdOutcome};

#[async_trait]
pub trait Cmd: Send + Sync {
    async fn update_ctx(&self, _ctx: &mut Ctx) -> Result<(), CliError> {
        Ok(())
    }

    async fn run(&self, _ctx: &mut Ctx) -> Result<CmdOutcome, CliError> {
        Ok(CmdOutcome::Success)
    }

    fn next_cmd(&self) -> Option<&dyn Cmd> {
        None
    }
}

impl<'ctx> dyn Cmd + 'ctx {
    pub fn walk_execute(
        &'ctx self,
        ctx: &'ctx mut Ctx,
    ) -> Pin<Box<dyn Future<Output = Result<CmdOutcome, CliError>> + Send + 'ctx>> {
        Box::pin(async move {
            self.update_ctx(ctx).await?;

            let outcome = self.run(ctx).await?;

            // A failing command stops the walk so its subcommand never runs on a
            // state the parent rejected.
            if !outcome.is_success() {
                return Ok(outcome);
            }

            match self.next_cmd() {
                Some(next) => next.walk_execute(ctx).await,
                None => Ok(outcome),
            }
        })
    }
}
