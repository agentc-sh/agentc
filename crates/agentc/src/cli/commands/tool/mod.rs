// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod init;

use async_trait::async_trait;
use clap::Subcommand;

use crate::cli::traits::Cmd;

#[derive(clap::Args, Debug, Clone)]
pub struct CliCommandTool {
    #[clap(subcommand)]
    pub command: ToolCommands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ToolCommands {
    #[clap(name = "init", about = "Scaffold a new tool package")]
    Init(init::CliCommandToolInit),
}

#[async_trait]
impl Cmd for CliCommandTool {
    fn next_cmd(&self) -> Option<&dyn Cmd> {
        match &self.command {
            ToolCommands::Init(cmd) => Some(cmd as &dyn Cmd),
        }
    }
}
