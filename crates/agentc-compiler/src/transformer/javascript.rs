// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

use crate::{
    asset::types::AssetOrigin,
    transformer::{
        errors::TransformError,
        traits::{AssetTransformer, TransformSink},
        types::AssetArtifact,
    },
    utils::command_exists,
};

pub struct JavascriptTransformer;

impl Default for JavascriptTransformer {
    fn default() -> Self {
        Self::new()
    }
}

impl JavascriptTransformer {
    pub fn new() -> Self {
        Self
    }

    async fn check_tools(&self) -> Result<(), TransformError> {
        if !command_exists("node").await {
            return Err(TransformError::tool_not_found(
                "node",
                "node is required to transform JavaScript/TypeScript assets.",
            ));
        }

        if !command_exists("npx").await {
            return Err(TransformError::tool_not_found(
                "npx",
                "npx is required to run esbuild. It is included with Node.js >= 5.2.0",
            ));
        }

        Ok(())
    }

    async fn run_with_sink(
        cmd: &mut Command,
        sink: &dyn TransformSink,
        err_path: &Path,
        err_msg: &str,
    ) -> Result<(), TransformError> {
        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| TransformError::io(err_path.to_string_lossy(), e))?;

        let mut stdout = BufReader::new(child.stdout.take().unwrap()).lines();
        let mut stderr = BufReader::new(child.stderr.take().unwrap()).lines();

        loop {
            tokio::select! {
                Ok(Some(line)) = stdout.next_line() => sink.stdout(&line).await,
                Ok(Some(line)) = stderr.next_line() => sink.stderr(&line).await,
                else => break,
            }
        }

        child
            .wait()
            .await
            .map_err(|e| TransformError::io(err_path.to_string_lossy(), e))
            .and_then(|status| {
                status
                    .success()
                    .then_some(())
                    .ok_or_else(|| TransformError::failed(err_path.to_string_lossy(), err_msg))
            })
    }

    async fn transform_package(
        &self,
        dir: &Path,
        sink: &dyn TransformSink,
    ) -> Result<Vec<AssetArtifact>, TransformError> {
        Self::run_with_sink(
            Command::new("npm")
                .arg("install")
                .current_dir(dir),
            sink,
            dir,
            "npm install failed",
        )
        .await?;

        let entry = self.detect_entry(dir).await?;

        let output_path = dir
            .join("dist")
            .join(
                entry
                    .strip_prefix(dir)
                    .expect("entry should be inside the package directory"),
            )
            .with_extension("js");

        Self::run_with_sink(
            Command::new("npx")
                .args([
                    "esbuild",
                    entry.to_str().unwrap(),
                    "--bundle",
                    "--platform=browser",
                    "--format=esm",
                    &format!("--outfile={}", output_path.to_str().unwrap()),
                ])
                .current_dir(dir),
            sink,
            dir,
            "esbuild bundling failed",
        )
        .await?;

        Ok(vec![AssetArtifact::path("source", output_path)])
    }

    async fn transform_file(
        &self,
        path: &Path,
        sink: &dyn TransformSink,
    ) -> Result<Vec<AssetArtifact>, TransformError> {
        let output_path = path.with_extension("js");

        Self::run_with_sink(
            Command::new("npx").args([
                "esbuild",
                path.to_str().unwrap(),
                "--bundle",
                "--platform=browser",
                "--format=esm",
                &format!("--outfile={}", output_path.to_str().unwrap()),
            ]),
            sink,
            path,
            "esbuild bundling failed",
        )
        .await?;

        Ok(vec![AssetArtifact::path("source", output_path)])
    }

    async fn detect_entry(&self, dir: &Path) -> Result<PathBuf, TransformError> {
        // Prefer index.ts, fall back to index.js, then check package.json main field
        for candidate in &["index.ts", "index.js", "index.tsx", "index.jsx"] {
            let path = dir.join(candidate);

            if tokio::fs::try_exists(&path)
                .await
                .unwrap_or(false)
            {
                return Ok(path);
            }
        }

        // Try package.json main field
        let pkg_path = dir.join("package.json");
        let pkg = serde_json::from_str::<serde_json::Value>(
            &tokio::fs::read_to_string(&pkg_path)
                .await
                .map_err(|e| TransformError::io(pkg_path.to_string_lossy(), e))?,
        )
        .map_err(|_| {
            TransformError::failed(dir.to_string_lossy(), "failed to parse package.json")
        })?;

        if let Some(main) = pkg["main"].as_str() {
            let path = dir.join(main);

            if tokio::fs::try_exists(&path)
                .await
                .unwrap_or(false)
            {
                return Ok(path);
            }
        }

        Err(TransformError::failed(
            dir.to_string_lossy(),
            "could not detect entry point; expected index.ts, index.js, or package.json main field",
        ))
    }
}

#[async_trait]
impl AssetTransformer for JavascriptTransformer {
    async fn can_transform(&self, local_path: &Path, _origin: &AssetOrigin) -> bool {
        if local_path
            .extension()
            .is_some_and(|e| {
                ["ts", "tsx", "js", "jsx"].contains(
                    &e.to_string_lossy()
                        .to_lowercase()
                        .as_str(),
                )
            })
        {
            return true;
        }

        if local_path.is_dir() {
            return tokio::fs::try_exists(local_path.join("package.json"))
                .await
                .unwrap_or(false);
        }

        false
    }

    async fn transform(
        &self,
        local_path: &Path,
        _origin: &AssetOrigin,
        sink: &dyn TransformSink,
    ) -> Result<Vec<AssetArtifact>, TransformError> {
        self.check_tools().await?;

        if local_path.is_dir() {
            self.transform_package(local_path, sink)
                .await
        } else {
            self.transform_file(local_path, sink)
                .await
        }
    }
}
