// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use clap::{ArgAction, Args};
use std::path::{PathBuf, absolute};

use agentc_core::{
    build::{pipeline::BuildPipeline, types::BuildParams},
    compiler::{
        asset::{ArtifactStore, AssetResolver, LocalFileHandler},
        generator::loader::FileSystemLoader,
        transformer::TransformerRegistry,
    },
    manifest::Manifest,
    parser::{
        SpecFormat, SpecParser,
        middleware::hcl::{
            ExpressionFunctionDeserialize, FileFunctionDeserialize, RootedFileReader,
            RuntimeFunctionDeserialize,
        },
    },
};

use crate::cli::{
    catalog::DefaultCompilationCatalog,
    commands::build::renderer::BuildStreamRenderer,
    context::Ctx,
    errors::CliError,
    project::ProjectArgs,
    traits::Cmd,
    types::CmdOutcome,
    ui::{StreamRenderer, UiFormat},
};

#[derive(Clone, Args, Debug)]
pub struct CliCommandBuild {
    #[clap(flatten)]
    project: ProjectArgs,
    /// The output directory where the compiled binary will be saved.
    #[clap(short, long)]
    output: Option<PathBuf>,
    /// Build in release mode.
    #[clap(long, action = ArgAction::SetTrue)]
    release: bool,
    /// Enable verbose logging for debugging purposes.
    #[clap(short, long, action = ArgAction::SetTrue)]
    verbose: bool,
    /// Set the UI format.
    #[clap(long, default_value = "auto")]
    format: UiFormat,
    /// Additional arguments to pass to the build process.
    #[clap(last = true)]
    args: Vec<String>,
    /// Skip cleanup of ephemeral build artifacts (e.g. temporary venvs) after compilation.
    #[clap(long, action = ArgAction::SetTrue)]
    no_cleanup: bool,
    /// Override the directory used for compiler caches (e.g. cargo target dir).
    #[clap(long)]
    cache_dir: Option<PathBuf>,
    /// Skip reading and writing the compiler cache for this build.
    #[clap(long, action = ArgAction::SetTrue)]
    no_cache: bool,
}

#[async_trait]
impl Cmd for CliCommandBuild {
    async fn run(&self, _ctx: &mut Ctx) -> Result<CmdOutcome, CliError> {
        let project = self.project.resolve().await?;

        let manifest = SpecParser::<Manifest>::default()
            .with_file_format(
                project.manifest().to_string_lossy(),
                SpecFormat::hcl()
                    .with_hcl_deserialize_middleware(FileFunctionDeserialize::new(
                        RootedFileReader::new(project.context().to_path_buf()),
                    ))
                    .with_hcl_deserialize_middleware(RuntimeFunctionDeserialize)
                    .with_hcl_deserialize_middleware(ExpressionFunctionDeserialize),
            )
            .parse()
            .await
            .map_err(|e| CliError::unexpected_error(e.to_string()))?;

        let (pipeline, mut rx) = BuildPipeline::builder()
            .manifest(manifest)
            .params(BuildParams {
                output_dir: match &self.output {
                    Some(dir) => {
                        absolute(dir).map_err(|e| CliError::unexpected_error(e.to_string()))?
                    }
                    None => project.artifacts_dir().join("build"),
                },
                target_dir: project
                    .artifacts_dir()
                    .join("generated"),
                runtime_dir: project.artifacts_dir().join("runtime"),
                cache_dir: Some(match &self.cache_dir {
                    Some(dir) => {
                        absolute(dir).map_err(|e| CliError::unexpected_error(e.to_string()))?
                    }
                    None => project.artifacts_dir().join("cache"),
                }),
                no_cache: self.no_cache,
                release: self.release,
                verbose: self.verbose,
                args: self.args.clone(),
            })
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
            .skip_cleanup(self.no_cleanup)
            .build()
            .map_err(|e| CliError::unexpected_error(e.to_string()))?;

        let mut renderer = BuildStreamRenderer::new(self.format.ui(), self.verbose);

        let (result, _) = tokio::join!(async { pipeline.run().await }, async {
            while let Some(event) = rx.recv().await {
                renderer.on_event(&event);
            }
        });

        match result {
            Ok(_) => {
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
