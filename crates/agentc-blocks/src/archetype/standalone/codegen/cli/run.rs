// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use std::path::PathBuf;

use agentc_compiler::generator::{
    blocks::codegen::CodeGen, context::GenerationContext, errors::GeneratorError,
    extension::ExtensionRegistry,
};

use crate::context::ResolvedContext;

pub struct CliRunCodeGen;

impl CodeGen<ResolvedContext> for CliRunCodeGen {
    fn generate_files(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        _registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError> {
        let source = quote! {
            use std::sync::Arc;
            use anyhow::Result;
            use serde_json::{json, to_string};
            use clap::{Args, ValueEnum};
            use uuid::Uuid;
            use futures::stream::StreamExt;
            use tokio_util::sync::CancellationToken;

            use agentc_agent_react::service::{
                ApplicationService,
                operations::run::RunOperations,
                types::{
                    run::{RunParams, RunEvent},
                    message::{CreateMessageParams, CreateUserMessageParams},
                },
            };

            use crate::config::Config;
            use crate::agent::build_agent;

            #[derive(Clone, Debug, ValueEnum)]
            pub enum RunFormat {
                /// Human-readable output for interactive use.
                Human,
                /// One JSON object per line for programmatic consumption.
                Json,
            }

            #[derive(Args, Debug)]
            pub struct RunArgs {
                /// The input message to send to the agent.
                pub input: String,
                /// The tenant ID to use for the conversation (optional).
                #[arg(long)]
                pub tenant_id: Option<String>,
                /// The session ID to use for the conversation (optional).
                #[arg(long)]
                pub session_id: Option<Uuid>,
                /// The run ID to use for the conversation (optional).
                #[arg(long)]
                pub run_id: Option<Uuid>,
                /// Output format.
                #[arg(long, default_value = "human")]
                pub format: RunFormat,
            }

            pub async fn run(args: RunArgs) -> Result<()> {
                let shutdown = CancellationToken::new();

                let config = Config::load().await?;
                let tenant_id = args.tenant_id
                    .clone()
                    .unwrap_or_else(|| config.default_tenant_id.clone());
                let session_id = args.session_id
                    .unwrap_or_else(Uuid::new_v4);
                let run_id = args.run_id
                    .unwrap_or_else(Uuid::new_v4);

                let database = Arc::new(config.database.build(true).await?);
                let agent = build_agent(database.clone(), &config, shutdown).await?;
                let service = ApplicationService::builder()
                    .with_agent(agent)
                    .with_database(database)
                    .build();

                let (mut stream, handle) = service
                    .run(
                        RunParams::new(tenant_id, session_id)
                            .with_run_id(run_id)
                            .with_messages([
                                CreateMessageParams::User(CreateUserMessageParams::new(args.input))
                            ])
                    )
                    .await?;

                while let Some(event) = stream.next().await {
                    match args.format {
                        RunFormat::Json => {
                            println!("{}", match event {
                                RunEvent::RunFinished {
                                    timestamp,
                                    session_id,
                                    run_id,
                                    status,
                                    interrupt_payload,
                                    result,
                                } => to_string(&json!({
                                    "type": "run_finished",
                                    "timestamp": timestamp,
                                    "session_id": session_id,
                                    "run_id": run_id,
                                    "status": status,
                                    "interrupt_payload": interrupt_payload,
                                    "result": result.as_ref().map(|r| r.context.clone()),
                                }))?,
                                RunEvent::StateSnapshot { timestamp, state } => to_string(&json!({
                                    "type": "state_snapshot",
                                    "timestamp": timestamp,
                                    "state": state.context,
                                }))?,
                                RunEvent::StateDelta { timestamp, delta } => to_string(&json!({
                                    "type": "state_delta",
                                    "timestamp": timestamp,
                                    "delta": delta.context,
                                }))?,
                                _ => to_string(&event)?,
                            })
                        }
                        RunFormat::Human => match event {
                            RunEvent::ReasoningStart { .. } => print!("\n[Thinking]:\n"),
                            RunEvent::ReasoningMessageContent { delta, .. } => print!("{delta}"),
                            RunEvent::ReasoningEnd { .. } => println!(),
                            RunEvent::TextMessageStart { .. } => print!("\n[Assistant]:\n---\n"),
                            RunEvent::TextMessageContent { delta, .. } => print!("{delta}"),
                            RunEvent::TextMessageEnd { .. } => println!(),
                            RunEvent::ToolCallStart { tool_name, .. } => print!("\n  \u{2192} {tool_name}("),
                            RunEvent::ToolCallArgs { delta, .. } => print!("{delta}"),
                            RunEvent::ToolCallEnd { .. } => {},
                            RunEvent::ToolCallResult { content, .. } => {
                                println!(")\n       = {}", to_string(&content).unwrap_or_default())
                            },
                            RunEvent::ToolCallError { error, .. } => {
                                println!(")\n       ! {error}")
                            },
                            RunEvent::RunFinished { status, .. } => {
                                println!("\n[Run complete: {status:?}]")
                            },
                            RunEvent::RunError { error, .. } => {
                                println!("\n[Run error: {error}]")
                            },
                            _ => {}
                        },
                    }
                }

                handle.await?;

                Ok(())
            }
        };

        Ok(vec![("src/cli/run.rs".into(), source)])
    }
}
