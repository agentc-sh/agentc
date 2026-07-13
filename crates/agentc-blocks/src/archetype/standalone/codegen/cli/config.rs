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

pub struct CliConfigCodeGen;

impl CodeGen<ResolvedContext> for CliConfigCodeGen {
    fn generate_files(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        _registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError> {
        let source = quote! {
            use anyhow::Result;
            use clap::{Args, ValueEnum};

            use crate::config::Config;

            #[derive(Clone, Debug, ValueEnum)]
            pub enum ConfigFormat {
                /// Human-readable output for interactive use.
                Human,
                /// Pretty-printed JSON for programmatic consumption.
                Json,
            }

            #[derive(Args, Debug)]
            pub struct ConfigArgs {
                /// Output format.
                #[arg(long, default_value = "human")]
                pub format: ConfigFormat,
            }

            pub async fn config(args: ConfigArgs) -> Result<()> {
                let config = Config::load().await?;

                match args.format {
                    ConfigFormat::Human => println!("{:#?}", config),
                    ConfigFormat::Json => {
                        println!("{}", serde_json::to_string_pretty(&config)?)
                    }
                }

                Ok(())
            }
        };

        Ok(vec![("src/cli/config.rs".into(), source)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn context() -> GenerationContext<ResolvedContext> {
        GenerationContext::new(
            serde_json::from_value(json!({
                "slug": "assistant",
                "agent_name": "assistant",
                "runtime": { "default_tenant_id": "default" },
                "providers": [],
                "agent": {
                    "version": "0.1.0",
                    "description": null,
                    "prompt": null,
                    "capabilities": null,
                    "capability_policy": null,
                    "model": { "provider": "anthropic", "name": "claude" }
                },
                "blocks": {},
                "tools": {},
                "skills": {},
                "http_server": null
            }))
            .unwrap(),
        )
    }

    #[test]
    fn config_command_defaults_to_redacted_human_format() {
        let source = CliConfigCodeGen
            .generate_files(&context(), &ExtensionRegistry::empty())
            .unwrap()[0]
            .1
            .to_string();

        assert!(source.contains("enum ConfigFormat"));
        assert!(source.contains("default_value = \"human\""));
        assert!(source.contains("Human => println ! (\"{:#?}\""));
        assert!(source.contains("Json =>") && source.contains("to_string_pretty"));
    }
}
