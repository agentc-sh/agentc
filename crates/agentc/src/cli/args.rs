// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use anyhow::Result;
use async_trait::async_trait;
use clap::{CommandFactory, Parser, Subcommand};
use rustls::crypto::aws_lc_rs;

use crate::cli::{commands::*, context::Ctx, errors::CliError, traits::Cmd};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Parser, Debug)]
#[clap(
    name = "agentc",
    version,
    author = "Tim Pogue",
    about = "An LLM agent compiler and runtime."
)]
pub struct CliArgs {
    /// Print the full command reference as markdown and exit.
    #[clap(long)]
    pub help_detail: bool,

    #[clap(subcommand)]
    pub command: Option<CliCommands>,
}

impl CliArgs {
    pub async fn build_ctx(&self) -> Result<Ctx, CliError> {
        Ok(Ctx::default())
    }

    pub async fn run() {
        let _ = aws_lc_rs::default_provider().install_default();
        let args = Self::parse();

        if args.help_detail {
            print!(
                "{}",
                clap_markdown::help_markdown_custom::<CliArgs>(
                    &clap_markdown::MarkdownOptions::new().show_footer(false)
                )
            );
            return;
        }

        match &args.command {
            Some(command) => {
                let mut ctx = args
                    .build_ctx()
                    .await.unwrap_or_else(|e| e.exit());

                (command as &dyn Cmd)
                    .walk_execute(&mut ctx)
                    .await
                    .unwrap_or_else(|e| e.exit());
            }
            _ => {
                Self::command()
                    .print_help()
                    .map_err(CliError::from)
                    .unwrap_or_else(|e| e.exit());

                std::process::exit(1);
            }
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum CliCommands {
    #[clap(name = "completions", about = "Generate shell completions")]
    Completions(completions::CliCommandCompletions),
    #[clap(
        name = "generate",
        about = "Generate the source code for an agent from a manifest"
    )]
    Generate(generate::CliCommandGenerate),
    #[clap(name = "build", about = "Build an agent from a manifest")]
    Build(build::CliCommandBuild),
    #[clap(name = "inspect", about = "Inspect the resolved manifest of an agent")]
    Inspect(inspect::CliCommandInspect),
    #[clap(name = "init", about = "Scaffold a new agent project")]
    Init(init::CliCommandInit),
    #[clap(name = "tool", about = "Tool management commands")]
    Tool(tool::CliCommandTool),
}

#[async_trait]
impl Cmd for CliCommands {
    fn next_cmd(&self) -> Option<&dyn Cmd> {
        match self {
            CliCommands::Completions(cmd) => Some(cmd as &dyn Cmd),
            CliCommands::Generate(cmd) => Some(cmd as &dyn Cmd),
            CliCommands::Build(cmd) => Some(cmd as &dyn Cmd),
            CliCommands::Inspect(cmd) => Some(cmd as &dyn Cmd),
            CliCommands::Init(cmd) => Some(cmd as &dyn Cmd),
            CliCommands::Tool(cmd) => Some(cmd as &dyn Cmd),
        }
    }
}
