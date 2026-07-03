// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use clap::Args;
use std::path::PathBuf;

use agentc_core::{
    compiler::{
        asset::{ArtifactStore, AssetResolver, LocalFileHandler},
        generator::loader::FileSystemLoader,
        transformer::TransformerRegistry,
    },
    inspect::pipeline::InspectPipeline,
    manifest::Manifest,
    parser::{SpecFormat, SpecParser, middleware::hcl::RuntimeFunctionDeserialize},
};

use crate::cli::{context::Ctx, errors::CliError, traits::Cmd, ui::UiFormat};

#[derive(Clone, Args, Debug)]
pub struct CliCommandInspect {
    /// The directory path containing the agent manifest file.
    #[clap(default_value = ".")]
    context: PathBuf,
    /// Set the UI format.
    #[clap(long, default_value = "auto")]
    format: UiFormat,
    /// Output the raw manifest instead of resolved context.
    #[clap(long)]
    raw: bool,
}

#[async_trait]
impl Cmd for CliCommandInspect {
    async fn run(&self, _ctx: &mut Ctx) -> Result<(), CliError> {
        if !self.context.is_dir() && !self.context.join("agent.acl").is_file() {
            return Err(CliError::invalid_parameters(format!(
                "Context path '{}' must be a directory containing 'agent.acl'",
                self.context.to_string_lossy()
            )));
        }

        let context = self
            .context
            .canonicalize()
            .map_err(|e| CliError::unexpected_error(e.to_string()))?;

        let manifest = SpecParser::<Manifest>::default()
            .with_file_format(
                context
                    .join("agent.acl")
                    .to_string_lossy(),
                SpecFormat::hcl().with_hcl_deserialize_middleware(RuntimeFunctionDeserialize),
            )
            .parse()
            .await
            .map_err(|e| CliError::unexpected_error(e.to_string()))?;

        if self.raw {
            println!("{:#?}", manifest);
            return Ok(());
        }

        let (pipeline, mut rx) = InspectPipeline::builder()
            .manifest(manifest)
            .asset_resolver(
                AssetResolver::builder()
                    .with_store(ArtifactStore::new(context.join("artifacts").join("store")))
                    .with_handler(LocalFileHandler::new(context.clone()))
                    .build(),
            )
            .loader(FileSystemLoader::new(context.clone()))
            .transformer_registry(TransformerRegistry::default())
            .build()
            .map_err(|e| CliError::unexpected_error(e.to_string()))?;

        let (result, _) = tokio::join!(
            async {
                pipeline
                    .run()
                    .await
                    .map_err(|e| CliError::unexpected_error(e.to_string()))
            },
            async { while rx.recv().await.is_some() {} }
        );

        match result {
            Ok(res) => println!("{:#?}", res.context),
            Err(e) => self
                .format
                .ui()
                .failure(&format!("Inspection failed: {e}")),
        }

        Ok(())
    }
}
