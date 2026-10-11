// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use clap::Args;
use std::path::{Path, PathBuf, absolute};
use tokio::fs::{canonicalize, metadata};

use crate::cli::errors::CliError;

#[derive(Clone, Args, Debug)]
pub struct ProjectArgs {
    /// The directory path containing the agent manifest file.
    #[clap(default_value = ".")]
    context: PathBuf,
    /// The manifest file to read. Defaults to 'agent.acl' in the context directory.
    #[clap(long, value_name = "FILE")]
    manifest: Option<PathBuf>,
    /// The directory compiler artifacts are written to. Defaults to 'artifacts' in the context directory.
    #[clap(long, value_name = "DIR")]
    artifacts_dir: Option<PathBuf>,
}

impl ProjectArgs {
    pub async fn resolve(&self) -> Result<Project, CliError> {
        if !metadata(&self.context)
            .await
            .is_ok_and(|m| m.is_dir())
        {
            return Err(CliError::invalid_parameters(format!(
                "Context path '{}' must be a directory",
                self.context.to_string_lossy()
            )));
        }

        let context = canonicalize(&self.context)
            .await
            .map_err(|e| CliError::unexpected_error(e.to_string()))?;

        let manifest = self
            .manifest
            .clone()
            .unwrap_or_else(|| context.join("agent.acl"));

        if !metadata(&manifest)
            .await
            .is_ok_and(|m| m.is_file())
        {
            return Err(CliError::invalid_parameters(format!(
                "Manifest '{}' must be a file",
                manifest.to_string_lossy()
            )));
        }

        Ok(Project {
            manifest: canonicalize(&manifest)
                .await
                .map_err(|e| CliError::unexpected_error(e.to_string()))?,
            artifacts_dir: match &self.artifacts_dir {
                Some(dir) => {
                    absolute(dir).map_err(|e| CliError::unexpected_error(e.to_string()))?
                }
                None => context.join("artifacts"),
            },
            context,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Project {
    context: PathBuf,
    manifest: PathBuf,
    artifacts_dir: PathBuf,
}

impl Project {
    pub fn context(&self) -> &Path {
        &self.context
    }

    pub fn manifest(&self) -> &Path {
        &self.manifest
    }

    pub fn artifacts_dir(&self) -> &Path {
        &self.artifacts_dir
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env::{current_dir, temp_dir},
        fs::{create_dir_all, remove_dir_all, write},
        path::{Path, PathBuf},
    };
    use uuid::Uuid;

    use crate::cli::project::ProjectArgs;

    struct TempRoot {
        path: PathBuf,
    }

    impl TempRoot {
        fn new() -> Self {
            let path = temp_dir().join(format!("agentc-cli-project-{}", Uuid::new_v4()));

            create_dir_all(&path).unwrap();

            TempRoot { path: path.canonicalize().unwrap() }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = remove_dir_all(&self.path);
        }
    }

    #[tokio::test]
    async fn defaults_resolve_inside_the_context() {
        let root = TempRoot::new();

        write(root.path().join("agent.acl"), "").unwrap();

        let project = ProjectArgs {
            context: root.path().to_path_buf(),
            manifest: None,
            artifacts_dir: None,
        }
        .resolve()
        .await
        .unwrap();

        assert_eq!(project.context(), root.path());
        assert_eq!(project.manifest(), root.path().join("agent.acl"));
        assert_eq!(project.artifacts_dir(), root.path().join("artifacts"));
    }

    #[tokio::test]
    async fn a_manifest_outside_the_context_keeps_the_context() {
        let root = TempRoot::new();

        create_dir_all(root.path().join("context")).unwrap();
        write(root.path().join("prod.acl"), "").unwrap();

        let project = ProjectArgs {
            context: root.path().join("context"),
            manifest: Some(root.path().join("prod.acl")),
            artifacts_dir: None,
        }
        .resolve()
        .await
        .unwrap();

        assert_eq!(project.context(), root.path().join("context"));
        assert_eq!(project.manifest(), root.path().join("prod.acl"));
        assert_eq!(
            project.artifacts_dir(),
            root.path()
                .join("context")
                .join("artifacts")
        );
    }

    #[tokio::test]
    async fn a_relative_artifacts_dir_resolves_against_the_current_directory() {
        let root = TempRoot::new();

        write(root.path().join("agent.acl"), "").unwrap();

        let project = ProjectArgs {
            context: root.path().to_path_buf(),
            manifest: None,
            artifacts_dir: Some(PathBuf::from("out")),
        }
        .resolve()
        .await
        .unwrap();

        assert_eq!(project.artifacts_dir(), current_dir().unwrap().join("out"));
    }

    #[tokio::test]
    async fn a_context_that_is_not_a_directory_is_an_invalid_parameter() {
        let root = TempRoot::new();

        let error = ProjectArgs {
            context: root.path().join("missing"),
            manifest: None,
            artifacts_dir: None,
        }
        .resolve()
        .await
        .unwrap_err();

        assert_eq!(error.code(), 2);
    }

    #[tokio::test]
    async fn a_context_without_a_manifest_is_an_invalid_parameter() {
        let root = TempRoot::new();

        let error = ProjectArgs {
            context: root.path().to_path_buf(),
            manifest: None,
            artifacts_dir: None,
        }
        .resolve()
        .await
        .unwrap_err();

        assert_eq!(error.code(), 2);
    }
}
