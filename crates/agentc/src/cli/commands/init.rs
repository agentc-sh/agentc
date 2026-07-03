// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use clap::{ArgAction, Args};
use std::path::PathBuf;

use agentc_core::init::{InitAgent, InitAgentParams};

use crate::cli::{
    context::Ctx,
    errors::CliError,
    traits::Cmd,
    ui::{UiFormat, traits::Ui},
};

#[derive(Clone, Args, Debug)]
pub struct CliCommandInit {
    /// Name of the agent project to create.
    name: String,
    /// Directory to create the project in. Defaults to `./<name>`.
    #[clap(short, long)]
    directory: Option<PathBuf>,
    /// Overwrite existing agent.acl if present.
    #[clap(long, action = ArgAction::SetTrue)]
    force: bool,
    /// Set the UI format.
    #[clap(long, default_value = "auto")]
    format: UiFormat,
}

#[async_trait]
impl Cmd for CliCommandInit {
    async fn run(&self, _ctx: &mut Ctx) -> Result<(), CliError> {
        let dir = self
            .directory
            .clone()
            .unwrap_or_else(|| PathBuf::from(&self.name));

        if dir.join("agent.acl").exists() && !self.force {
            return Err(CliError::invalid_parameters(format!(
                "'{}' already contains an agent.acl; pass --force to overwrite",
                dir.display()
            )));
        }

        InitAgent::scaffold(InitAgentParams { name: self.name.clone() })
            .map_err(|e| CliError::unexpected_error(e.to_string()))?
            .write_to(&dir)
            .await?;

        self.format
            .ui()
            .success(&format!("Created {}", self.name));

        Ok(())
    }
}
