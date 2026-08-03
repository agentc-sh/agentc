// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::{path::PathBuf, process::Stdio};
use tokio::process::Command;

use crate::{
    artifact::ExecutableArtifact,
    runner::{
        errors::RunnerError,
        traits::Runner,
        types::{RunOutcome, RunParams},
    },
};

/// A resolved description of the process an invocation starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInvocation {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
}

/// Invokes an [`ExecutableArtifact`](crate::artifact::ExecutableArtifact) as a child
/// process attached to the current terminal.
pub struct ProcessRunner;

impl Default for ProcessRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessRunner {
    pub fn new() -> Self {
        Self
    }

    fn invocation(&self, artifact: &ExecutableArtifact, params: &RunParams) -> ProcessInvocation {
        ProcessInvocation {
            program: artifact.path.clone(),
            args: params.args.clone(),
            working_dir: params.context_dir.clone(),
        }
    }
}

#[async_trait]
impl Runner for ProcessRunner {
    type Artifact = ExecutableArtifact;

    async fn run(
        &self,
        artifact: &Self::Artifact,
        params: RunParams,
    ) -> Result<RunOutcome, RunnerError> {
        if !tokio::fs::try_exists(&artifact.path)
            .await
            .unwrap_or(false)
        {
            return Err(RunnerError::ArtifactMissing(artifact.path.clone()));
        }

        let invocation = self.invocation(artifact, &params);

        // Inheriting stdio hands the terminal to the child, which keeps its output
        // unbuffered and its input interactive. On unix it also delivers Ctrl+C
        // through the foreground process group, so no signal handling is needed here.
        Ok(
            RunOutcome::new(
                Command::new(&invocation.program)
                    .current_dir(&invocation.working_dir)
                    .args(&invocation.args)
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()?
                    .wait()
                    .await?
                    .code(),
            )
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact() -> ExecutableArtifact {
        ExecutableArtifact::new("/artifacts/build/assistant")
    }

    #[test]
    fn invocation_program_is_the_artifact_path() {
        assert_eq!(
            ProcessRunner::new()
                .invocation(&artifact(), &RunParams::new("/project"))
                .program,
            PathBuf::from("/artifacts/build/assistant"),
        );
    }

    #[test]
    fn invocation_forwards_arguments_verbatim_and_in_order() {
        assert_eq!(
            ProcessRunner::new()
                .invocation(
                    &artifact(),
                    &RunParams::new("/project").with_args(["run", "hello", "--format", "json"]),
                )
                .args,
            vec!["run", "hello", "--format", "json"],
        );
    }

    #[test]
    fn invocation_adds_no_arguments_of_its_own() {
        assert!(
            ProcessRunner::new()
                .invocation(&artifact(), &RunParams::new("/project"))
                .args
                .is_empty()
        );
    }

    #[test]
    fn invocation_working_dir_is_the_manifest_context() {
        assert_eq!(
            ProcessRunner::new()
                .invocation(&artifact(), &RunParams::new("/project"))
                .working_dir,
            PathBuf::from("/project"),
        );
    }
}
