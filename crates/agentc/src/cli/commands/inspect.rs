// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use clap::Args;

use agentc_core::{
    compiler::{
        asset::{ArtifactStore, AssetResolver, LocalFileHandler},
        generator::loader::FileSystemLoader,
        transformer::TransformerRegistry,
    },
    inspect::pipeline::InspectPipeline,
    manifest::Manifest,
    parser::{
        SpecFormat, SpecParser,
        middleware::hcl::{FileFunctionDeserialize, RootedFileReader, RuntimeFunctionDeserialize},
    },
};

use crate::cli::{
    catalog::DefaultCompilationCatalog, context::Ctx, errors::CliError, project::ProjectArgs,
    traits::Cmd, types::CmdOutcome, ui::UiFormat,
};

#[derive(Clone, Args, Debug)]
pub struct CliCommandInspect {
    #[clap(flatten)]
    project: ProjectArgs,
    /// Set the UI format.
    #[clap(long, default_value = "auto")]
    format: UiFormat,
    /// Output the raw manifest instead of resolved context.
    #[clap(long)]
    raw: bool,
}

#[async_trait]
impl Cmd for CliCommandInspect {
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

        if self.raw {
            println!("{:#?}", manifest);
            return Ok(CmdOutcome::Success);
        }

        let (pipeline, mut rx) = InspectPipeline::builder()
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
            Ok(res) => {
                println!("archetype: {}", res.archetype_name);
                println!("graph: {}", res.graph_name);
                if !res.protocol_names.is_empty() {
                    println!("protocols: {}", res.protocol_names.join(", "));
                }
                println!("{:#?}", res.context);

                Ok(CmdOutcome::Success)
            }
            Err(e) => {
                self.format
                    .ui()
                    .failure(&format!("Inspection failed: {e}"));

                Ok(CmdOutcome::failure(1))
            }
        }
    }
}
