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
    compiler::{
        errors::CompilerError,
        traits::{Compiler, OutputSink},
        types::{Artifact, CompileParams},
    },
    utils::command_exists,
};

pub struct CargoCompiler;

impl Default for CargoCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl CargoCompiler {
    pub fn new() -> Self {
        Self
    }

    async fn extract_binary_name(project_dir: &Path) -> Result<String, CompilerError> {
        let cargo_toml_path = project_dir.join("Cargo.toml");
        let cargo_toml_content = tokio::fs::read_to_string(&cargo_toml_path)
            .await
            .map_err(|e| {
                CompilerError::compilation_failed_sourced(
                    format!("Failed to read Cargo.toml at {}", cargo_toml_path.display()),
                    Some(e),
                )
            })?;

        let cargo_toml: toml::Value = toml::from_str(&cargo_toml_content).map_err(|e| {
            CompilerError::compilation_failed_sourced("Failed to parse Cargo.toml", Some(e))
        })?;

        cargo_toml
            .get("package")
            .and_then(|pkg| pkg.get("name"))
            .and_then(|name| name.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                CompilerError::compilation_failed("Failed to extract binary name from Cargo.toml")
            })
    }

    pub async fn is_installed() -> bool {
        command_exists("cargo").await
            && command_exists("rustup").await
    }

    pub async fn is_target_available(target: &str) -> Result<bool, CompilerError> {
        Ok(String::from_utf8(
            Command::new("rustup")
                .arg("target")
                .arg("list")
                .arg("--installed")
                .output()
                .await?
                .stdout,
        )
        .map_err(|e| {
            CompilerError::compilation_failed_sourced("Failed to read rustup output", Some(e))
        })?
        .lines()
        .any(|line| line == target))
    }
}

#[async_trait]
impl Compiler for CargoCompiler {
    async fn compile(
        &self,
        params: CompileParams,
        output_sink: &dyn OutputSink,
    ) -> Result<Artifact, CompilerError> {
        if !Self::is_installed().await {
            return Err(CompilerError::compilation_failed(
                "Cargo or rustup is not installed. Please install it from https://www.rust-lang.org/tools/install.",
            ));
        }

        if let Some(target) = &params.target
            && !Self::is_target_available(target).await?
        {
            return Err(CompilerError::compilation_failed(format!(
                "Target '{}' is not installed. Please install it with 'rustup target add {}'.",
                target, target
            )));
        }

        let output_dir = params
            .cache_dir
            .as_deref()
            .unwrap_or(&params.project_dir.join("target"))
            .to_string_lossy()
            .to_string();

        let mut child = Command::new("cargo")
            .current_dir(&params.project_dir)
            .arg("build")
            .args(
                params
                    .release
                    .then_some(vec!["--release"])
                    .unwrap_or_default(),
            )
            .args(vec!["--target-dir", &output_dir])
            .args(
                params
                    .target
                    .as_ref()
                    .map(|target| vec!["--target", target.as_str()])
                    .unwrap_or_default(),
            )
            .arg(if params.verbose {
                "--message-format=human"
            } else {
                "--message-format=short"
            })
            .args(params.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let mut stdout = BufReader::new(child.stdout.take().unwrap()).lines();
        let mut stderr = BufReader::new(child.stderr.take().unwrap()).lines();

        loop {
            tokio::select! {
                Ok(Some(line)) = stdout.next_line() => output_sink.stdout(&line).await,
                Ok(Some(line)) = stderr.next_line() => output_sink.stderr(&line).await,
                else => break,
            }
        }

        child
            .wait()
            .await
            .map_err(|e| {
                CompilerError::compilation_failed_sourced(
                    "Failed to wait for cargo process",
                    Some(e),
                )
            })
            .and_then(|status| {
                status
                    .success()
                    .then_some(())
                    .ok_or_else(|| {
                        CompilerError::compilation_failed(format!(
                            "cargo exited with status code {}",
                            status.code().unwrap_or(-1)
                        ))
                    })
            })?;

        let binary_name = Self::extract_binary_name(&params.project_dir).await?;
        let profile = if params.release { "release" } else { "debug" };
        let is_windows = params
            .target
            .as_deref()
            .map_or(cfg!(target_os = "windows"), |t| t.contains("windows"));

        let binary_path = PathBuf::from(&output_dir)
            .join(params.target.as_deref().unwrap_or(""))
            .join(profile)
            .join(&binary_name)
            .with_extension(if is_windows { "exe" } else { "" });

        tokio::fs::create_dir_all(&params.target_dir)
            .await
            .map_err(|e| {
                CompilerError::compilation_failed_sourced(
                    format!("Failed to create target directory at {}", params.target_dir.display()),
                    Some(e),
                )
            })?;

        tokio::fs::copy(
            &binary_path,
            params
                .target_dir
                .join(binary_path.file_name().unwrap()),
        )
        .await
        .map_err(|e| {
            CompilerError::compilation_failed_sourced(
                format!(
                    "Failed to copy binary from {} to {}",
                    binary_path.display(),
                    params.target_dir.display()
                ),
                Some(e),
            )
        })?;

        Ok(Artifact { target_dir: params.target_dir })
    }
}
