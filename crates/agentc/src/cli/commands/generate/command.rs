// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use clap::{ArgAction, Args};
use std::path::PathBuf;

use agentc_core::{
    blocks::archetype::{resolver::ArchetypeResolver, standalone::StandaloneArchetype},
    compiler::{
        asset::{ArtifactStore, AssetResolver, LocalFileHandler},
        generator::loader::FileSystemLoader,
        transformer::TransformerRegistry,
    },
    generate::pipeline::GeneratePipeline,
    manifest::Manifest,
    parser::{SpecFormat, SpecParser, middleware::hcl::RuntimeFunctionDeserialize},
};

use crate::cli::{
    commands::generate::renderer::GenerateStreamRenderer,
    context::Ctx,
    errors::CliError,
    traits::Cmd,
    ui::{StreamRenderer, UiFormat},
};

#[derive(Clone, Args, Debug)]
pub struct CliCommandGenerate {
    /// The directory path containing the agent manifest file.
    #[clap(default_value = ".")]
    context: PathBuf,
    /// The output directory where the generated agent files will be saved.
    #[clap(short, long)]
    output: Option<PathBuf>,
    /// Dry run the generation process without actually writing files to disk.
    #[clap(long, action = ArgAction::SetTrue)]
    dry_run: bool,
    /// Set the UI format.
    #[clap(long, default_value = "auto")]
    format: UiFormat,
    /// Enable verbose logging for debugging purposes.
    #[clap(short, long, action = ArgAction::SetTrue)]
    verbose: bool,
    /// Clean up ephemeral build artifacts (e.g. temporary venvs) after generation.
    #[clap(long, action = ArgAction::SetTrue)]
    cleanup: bool,
}

#[async_trait]
impl Cmd for CliCommandGenerate {
    async fn run(&self, _ctx: &mut Ctx) -> Result<(), CliError> {
        if !self.context.is_dir() || !self.context.join("agent.acl").is_file() {
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

        let (pipeline, mut rx) = GeneratePipeline::builder()
            .manifest(manifest)
            .asset_resolver(
                AssetResolver::builder()
                    .with_store(ArtifactStore::new(context.join("artifacts").join("store")))
                    .with_handler(LocalFileHandler::new(context.clone()))
                    .build(),
            )
            .loader(FileSystemLoader::new(context.clone()))
            .archetype_resolver(
                ArchetypeResolver::builder()
                    .with_archetype(StandaloneArchetype)
                    .build(),
            )
            .transformer_registry(TransformerRegistry::default())
            .runtime_dir(
                context
                    .join("artifacts")
                    .join("runtime"),
            )
            .cleanup(self.cleanup)
            .build()
            .map_err(|e| CliError::unexpected_error(e.to_string()))?;

        let mut renderer = GenerateStreamRenderer::new(self.format.ui(), self.verbose);

        let (result, _) = tokio::join!(
            async {
                pipeline
                    .run()
                    .await
                    .map_err(|e| CliError::unexpected_error(e.to_string()))
            },
            async {
                while let Some(event) = rx.recv().await {
                    renderer.on_event(&event);
                }
            }
        );

        match result {
            Ok(res) => {
                if self.dry_run {
                    for (path, content) in res.vfs.iter() {
                        println!("--- {:?} ---\n{}\n", path, content);
                        println!("--- End of {:?} ---\n", path);
                    }
                } else {
                    res.vfs
                        .write_to(
                            &self
                                .output
                                .as_ref()
                                .map(|p| {
                                    p.canonicalize()
                                        .unwrap_or_else(|_| p.clone())
                                })
                                .unwrap_or_else(|| {
                                    context
                                        .join("artifacts")
                                        .join("generated")
                                }),
                        )
                        .await
                        .map_err(|e| CliError::unexpected_error(e.to_string()))?;
                }

                renderer.on_success();
            }
            Err(e) => renderer.on_failure(&e.to_string()),
        }

        Ok(())
    }
}
