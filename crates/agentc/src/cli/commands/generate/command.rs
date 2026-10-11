// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use clap::{ArgAction, Args};
use std::path::{PathBuf, absolute};

use agentc_core::{
    compiler::{
        asset::{ArtifactStore, AssetResolver, LocalFileHandler},
        generator::loader::FileSystemLoader,
        transformer::TransformerRegistry,
    },
    generate::pipeline::GeneratePipeline,
    manifest::Manifest,
    parser::{
        SpecFormat, SpecParser,
        middleware::hcl::{FileFunctionDeserialize, RootedFileReader, RuntimeFunctionDeserialize},
    },
};

use crate::cli::{
    catalog::DefaultCompilationCatalog,
    commands::generate::renderer::GenerateStreamRenderer,
    context::Ctx,
    errors::CliError,
    project::ProjectArgs,
    traits::Cmd,
    types::CmdOutcome,
    ui::{StreamRenderer, UiFormat},
};

#[derive(Clone, Args, Debug)]
pub struct CliCommandGenerate {
    #[clap(flatten)]
    project: ProjectArgs,
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
    async fn run(&self, _ctx: &mut Ctx) -> Result<CmdOutcome, CliError> {
        let project = self.project.resolve().await?;

        let manifest = SpecParser::<Manifest>::default()
            .with_file_format(
                project.manifest().to_string_lossy(),
                SpecFormat::hcl()
                    .with_hcl_deserialize_middleware(FileFunctionDeserialize::new(
                        RootedFileReader::new(project.context().to_path_buf()),
                    ))
                    .with_hcl_deserialize_middleware(RuntimeFunctionDeserialize),
            )
            .parse()
            .await
            .map_err(|e| CliError::unexpected_error(e.to_string()))?;

        let (pipeline, mut rx) = GeneratePipeline::builder()
            .manifest(manifest)
            .asset_resolver(
                AssetResolver::builder()
                    .with_store(ArtifactStore::new(project.artifacts_dir().join("store")))
                    .with_handler(LocalFileHandler::new(project.context().to_path_buf()))
                    .build(),
            )
            .loader(FileSystemLoader::new(project.context().to_path_buf()))
            .catalog(
                DefaultCompilationCatalog::build()
                    .map_err(|e| CliError::unexpected_error(e.to_string()))?,
            )
            .transformer_registry(TransformerRegistry::default())
            .runtime_dir(project.artifacts_dir().join("runtime"))
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
                        .write_to(&match &self.output {
                            Some(dir) => absolute(dir)
                                .map_err(|e| CliError::unexpected_error(e.to_string()))?,
                            None => project
                                .artifacts_dir()
                                .join("generated"),
                        })
                        .await
                        .map_err(|e| CliError::unexpected_error(e.to_string()))?;
                }

                renderer.on_success();

                Ok(CmdOutcome::Success)
            }
            Err(e) => {
                renderer.on_failure(&e.to_string());

                Ok(CmdOutcome::failure(1))
            }
        }
    }
}
