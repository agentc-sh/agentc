// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::{
    fs,
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
    utils::{command_exists, symlink},
};

#[derive(Deserialize)]
struct PyProject {
    project: PyProjectMeta,
}

#[derive(Deserialize)]
struct PyProjectMeta {
    name: String,
}

pub struct PythonTransformer;

impl Default for PythonTransformer {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonTransformer {
    pub fn new() -> Self {
        Self
    }

    async fn check_tools(&self) -> Result<(), TransformError> {
        if !command_exists("uv").await {
            return Err(TransformError::tool_not_found(
                "uv",
                "uv is required to install Python tool dependencies. \
                 Install it from https://docs.astral.sh/uv/getting-started/installation/",
            ));
        }

        Ok(())
    }

    /// Read the importable module name from `[project].name` in `pyproject.toml`.
    ///
    /// PEP 625 normalizes distribution names by replacing hyphens with underscores, which
    /// is also the importable form. We apply the same normalization here.
    async fn read_module_name(&self, project_dir: &Path) -> Result<String, TransformError> {
        let toml_path = project_dir.join("pyproject.toml");
        let contents = fs::read_to_string(&toml_path)
            .await
            .map_err(|e| TransformError::io(toml_path.to_string_lossy(), e))?;

        let pyproject: PyProject = toml::from_str(&contents).map_err(|e| {
            TransformError::failed(
                project_dir.to_string_lossy(),
                format!("failed to parse pyproject.toml: {e}"),
            )
        })?;

        Ok(pyproject.project.name.replace('-', "_"))
    }

    /// Locate the package source directory inside the project.
    ///
    /// Supports both flat layout (`<project>/<module>/`) and src layout
    /// (`<project>/src/<module>/`).
    async fn find_source_dir(
        &self,
        project_dir: &Path,
        module_name: &str,
    ) -> Result<PathBuf, TransformError> {
        let flat = project_dir.join(module_name);
        if fs::try_exists(&flat)
            .await
            .unwrap_or(false)
        {
            return Ok(flat);
        }

        let src = project_dir
            .join("src")
            .join(module_name);
        if fs::try_exists(&src)
            .await
            .unwrap_or(false)
        {
            return Ok(src);
        }

        Err(TransformError::failed(
            project_dir.to_string_lossy(),
            format!(
                "could not locate source directory for module '{module_name}' -- \
                 expected '{module_name}/' or 'src/{module_name}/' inside the project"
            ),
        ))
    }

    /// Build a stable temporary directory path for this project.
    ///
    /// The path is derived from a SHA-256 hash of the canonical project directory path,
    /// ensuring the same project always maps to the same temp location across runs.
    /// This keeps the artifacts store symlinks valid until the temp dir is explicitly
    /// cleaned up after compilation.
    fn temp_dir_for(&self, project_dir: &Path) -> PathBuf {
        let hash = format!("{:x}", Sha256::digest(project_dir.to_string_lossy().as_bytes()));
        std::env::temp_dir().join(format!("agentc-python-{}", &hash[..16]))
    }

    /// Sync runtime dependencies into the isolated temp venv.
    ///
    /// `UV_PROJECT_ENVIRONMENT` points `uv sync` at the temp venv so the project's own
    /// `.venv` is never created or modified.
    async fn sync_venv(
        &self,
        project_dir: &Path,
        module_name: &str,
        venv_dir: &Path,
        sink: &dyn TransformSink,
    ) -> Result<(), TransformError> {
        let mut child = Command::new("uv")
            .args(["sync", "--reinstall-package", module_name, "--no-dev"])
            .current_dir(project_dir)
            .env("UV_PROJECT_ENVIRONMENT", venv_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| TransformError::io(project_dir.to_string_lossy(), e))?;

        let mut stdout = BufReader::new(child.stdout.take().unwrap()).lines();
        let mut stderr = BufReader::new(child.stderr.take().unwrap()).lines();

        loop {
            tokio::select! {
                Ok(Some(line)) = stdout.next_line() => sink.stdout(&line).await,
                Ok(Some(line)) = stderr.next_line() => sink.stderr(&line).await,
                else => break,
            }
        }

        let status = child
            .wait()
            .await
            .map_err(|e| TransformError::io(project_dir.to_string_lossy(), e))?;

        if !status.success() {
            return Err(TransformError::failed(
                project_dir.to_string_lossy(),
                "uv sync failed -- check that pyproject.toml is valid and all dependencies can be resolved",
            ));
        }

        Ok(())
    }

    /// Locate the `site-packages` directory inside a venv.
    ///
    /// On Unix the layout is `.venv/lib/pythonX.Y/site-packages`.
    /// On Windows the layout is `.venv/Lib/site-packages`.
    async fn find_site_packages(&self, venv_dir: &Path) -> Result<PathBuf, TransformError> {
        let windows_site_packages = venv_dir
            .join("Lib")
            .join("site-packages");
        if fs::try_exists(&windows_site_packages)
            .await
            .unwrap_or(false)
        {
            return Ok(windows_site_packages);
        }

        let lib_dir = venv_dir.join("lib");
        let mut entries = fs::read_dir(&lib_dir)
            .await
            .map_err(|e| TransformError::io(lib_dir.to_string_lossy(), e))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| TransformError::io(lib_dir.to_string_lossy(), e))?
        {
            if entry
                .file_name()
                .to_string_lossy()
                .starts_with("python")
            {
                let site_packages = entry.path().join("site-packages");
                if fs::try_exists(&site_packages)
                    .await
                    .unwrap_or(false)
                {
                    return Ok(site_packages);
                }
            }
        }

        Err(TransformError::failed(
            venv_dir.to_string_lossy(),
            "could not locate site-packages inside the venv -- \
             ensure uv sync completed successfully and a virtual environment was created",
        ))
    }

    /// Build the staging directory for `py_freeze`.
    ///
    /// The staging directory contains only symlinks to `pyproject.toml` and the package
    /// source directory. This prevents `py_freeze` from traversing into `.venv`,
    /// `__pycache__`, or any other files in the project root.
    async fn build_staging_dir(
        &self,
        staging_dir: &Path,
        project_dir: &Path,
        source_dir: &Path,
        module_name: &str,
    ) -> Result<(), TransformError> {
        if fs::try_exists(staging_dir)
            .await
            .unwrap_or(false)
        {
            fs::remove_dir_all(staging_dir)
                .await
                .map_err(|e| TransformError::io(staging_dir.to_string_lossy(), e))?;
        }

        fs::create_dir_all(staging_dir)
            .await
            .map_err(|e| TransformError::io(staging_dir.to_string_lossy(), e))?;

        symlink(
            &project_dir.join("pyproject.toml"),
            &staging_dir.join("pyproject.toml"),
        )
        .await
        .map_err(|e| TransformError::io(staging_dir.to_string_lossy(), e))?;

        symlink(source_dir, &staging_dir.join(module_name))
            .await
            .map_err(|e| TransformError::io(staging_dir.to_string_lossy(), e))?;

        Ok(())
    }
}

#[async_trait]
impl AssetTransformer for PythonTransformer {
    async fn can_transform(&self, local_path: &Path, _origin: &AssetOrigin) -> bool {
        if !local_path.is_dir() {
            return false;
        }

        tokio::fs::try_exists(local_path.join("pyproject.toml"))
            .await
            .unwrap_or(false)
    }

    async fn transform(
        &self,
        local_path: &Path,
        _origin: &AssetOrigin,
        sink: &dyn TransformSink,
    ) -> Result<Vec<AssetArtifact>, TransformError> {
        self.check_tools().await?;

        let module_name = self
            .read_module_name(local_path)
            .await?;
        let source_dir = self
            .find_source_dir(local_path, &module_name)
            .await?;

        let temp_root = self.temp_dir_for(local_path);
        let venv_dir = temp_root.join(".venv");
        let staging_dir = temp_root.join("staging");

        self.sync_venv(local_path, &module_name, &venv_dir, sink)
            .await?;

        let site_packages = self
            .find_site_packages(&venv_dir)
            .await?;

        self.build_staging_dir(&staging_dir, local_path, &source_dir, &module_name)
            .await?;

        Ok(vec![
            AssetArtifact::path("project_path", &staging_dir),
            AssetArtifact::path("site_packages_path", site_packages),
            AssetArtifact::value("module_name", module_name),
            // The entire temp root (venv + staging) is ephemeral and will be removed
            // by CleanupStep after compilation.
            AssetArtifact::ephemeral_path("temp_dir", temp_root),
        ])
    }
}
