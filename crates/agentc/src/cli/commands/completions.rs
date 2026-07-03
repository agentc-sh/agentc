// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::io;

use async_trait::async_trait;
use clap::{Args, CommandFactory};
use clap_complete::{Shell, generate};

use crate::cli::{args::CliArgs, context::Ctx, errors::CliError, traits::Cmd};

#[derive(Debug, Clone, Args)]
pub struct CliCommandCompletions {
    /// The shell to generate a completion script for.
    pub shell: Shell,
}

#[async_trait]
impl Cmd for CliCommandCompletions {
    async fn run(&self, _ctx: &mut Ctx) -> Result<(), CliError> {
        generate(self.shell, &mut CliArgs::command(), "agentc", &mut io::stdout());
        Ok(())
    }
}
