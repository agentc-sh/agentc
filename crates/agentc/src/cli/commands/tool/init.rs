// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use clap::{ArgAction, Args, ValueEnum};
use std::path::PathBuf;

use agentc_core::init::{InitTool, InitToolParams, ToolLanguage};

use crate::cli::{
    context::Ctx,
    errors::CliError,
    traits::Cmd,
    ui::{UiFormat, traits::Ui},
};

#[derive(Clone, Debug, ValueEnum)]
pub enum ToolLanguageCli {
    Python,
    Javascript,
}

impl From<ToolLanguageCli> for ToolLanguage {
    fn from(v: ToolLanguageCli) -> Self {
        match v {
            ToolLanguageCli::Python => ToolLanguage::Python,
            ToolLanguageCli::Javascript => ToolLanguage::Javascript,
        }
    }
}

#[derive(Clone, Args, Debug)]
pub struct CliCommandToolInit {
    /// Name of the tool package to create.
    name: String,
    /// Programming language for the tool.
    #[clap(short, long)]
    language: ToolLanguageCli,
    /// Directory to create the package in. Defaults to `./<name>`.
    #[clap(short, long)]
    directory: Option<PathBuf>,
    /// Overwrite if the target directory already exists and is non-empty.
    #[clap(long, action = ArgAction::SetTrue)]
    force: bool,
    /// Set the UI format.
    #[clap(long, default_value = "auto")]
    format: UiFormat,
}

#[async_trait]
impl Cmd for CliCommandToolInit {
    async fn run(&self, _ctx: &mut Ctx) -> Result<(), CliError> {
        let dir = self
            .directory
            .clone()
            .unwrap_or_else(|| PathBuf::from(&self.name));

        if dir.exists()
            && !self.force
            && !dir
                .read_dir()
                .map(|mut d| d.next().is_none())
                .unwrap_or(false)
        {
            return Err(CliError::invalid_parameters(format!(
                "'{}' already exists and is non-empty; pass --force to overwrite",
                dir.display()
            )));
        }

        InitTool::scaffold(InitToolParams {
            name: self.name.clone(),
            language: self.language.clone().into(),
        })
        .map_err(|e| CliError::unexpected_error(e.to_string()))?
        .write_to(&dir)
        .await?;

        self.format
            .ui()
            .success(&format!("Created {}", self.name));

        Ok(())
    }
}
