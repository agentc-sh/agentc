// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use clap::{ArgAction, Args};
use std::path::PathBuf;

use agentc_core::{
    compiler::{
        asset::{ArtifactStore, AssetResolver, LocalFileHandler},
        generator::loader::FileSystemLoader,
        transformer::TransformerRegistry,
    },
    manifest::Manifest,
    parser::{SpecFormat, SpecParser, middleware::hcl::RuntimeFunctionDeserialize},
    run::{pipeline::RunPipeline, types::RunParams},
};

use crate::cli::{
    catalog::DefaultCompilationCatalog,
    commands::run::renderer::RunStreamRenderer,
    context::Ctx,
    errors::CliError,
    traits::Cmd,
    types::CmdOutcome,
    ui::{StreamRenderer, UiFormat},
};

#[derive(Clone, Args, Debug)]
pub struct CliCommandRun {
    /// The directory path containing the agent manifest file.
    #[clap(default_value = ".")]
    context: PathBuf,
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
    #[clap(long = "build-arg", value_name = "ARG")]
    build_arg: Vec<String>,
    /// Arguments to pass to the built agent.
    #[clap(last = true)]
    args: Vec<String>,
    /// Skip cleanup of ephemeral build artifacts (e.g. temporary venvs) after compilation.
    #[clap(long, action = ArgAction::SetTrue)]
    no_cleanup: bool,
    /// Override the directory used for compiler caches (e.g. cargo target dir).
    #[clap(long)]
    cache_dir: Option<PathBuf>,
    /// Skip reading and writing the compiler cache for this run.
    #[clap(long, action = ArgAction::SetTrue)]
    no_cache: bool,
}

#[async_trait]
impl Cmd for CliCommandRun {
    async fn run(&self, _ctx: &mut Ctx) -> Result<CmdOutcome, CliError> {
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

        let (pipeline, mut rx) = RunPipeline::builder()
            .manifest(manifest)
            .params(RunParams {
                context_dir: context.clone(),
                output_dir: context.join("artifacts").join("build"),
                target_dir: context
                    .join("artifacts")
                    .join("generated"),
                runtime_dir: context
                    .join("artifacts")
                    .join("runtime"),
                cache_dir: Some(
                    self.cache_dir
                        .clone()
                        .unwrap_or_else(|| context.join("artifacts").join("cache")),
                ),
                no_cache: self.no_cache,
                release: self.release,
                verbose: self.verbose,
                build_args: self.build_arg.clone(),
                args: self.args.clone(),
            })
            .asset_resolver(
                AssetResolver::builder()
                    .with_store(ArtifactStore::new(context.join("artifacts").join("store")))
                    .with_handler(LocalFileHandler::new(context.clone()))
                    .build(),
            )
            .loader(FileSystemLoader::new(context.clone()))
            .catalog(
                DefaultCompilationCatalog::build()
                    .map_err(|e| CliError::unexpected_error(e.to_string()))?,
            )
            .transformer_registry(TransformerRegistry::default())
            .skip_cleanup(self.no_cleanup)
            .build()
            .map_err(|e| CliError::unexpected_error(e.to_string()))?;

        let mut renderer = RunStreamRenderer::new(self.format.ui(), self.verbose);

        let (result, _) = tokio::join!(async { pipeline.run().await }, async {
            while let Some(event) = rx.recv().await {
                renderer.on_event(&event);
            }
        });

        match result {
            Ok(result) => {
                renderer.on_success();

                match result.exit_code {
                    Some(0) => Ok(CmdOutcome::Success),
                    Some(code) => Ok(CmdOutcome::failure(code)),
                    // An invocation killed by a signal has no status of its own.
                    None => Ok(CmdOutcome::failure(1)),
                }
            }
            Err(e) => {
                renderer.on_failure(&e.to_string());

                Ok(CmdOutcome::failure(1))
            }
        }
    }
}
