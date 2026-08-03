// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::path::PathBuf;
use thiserror::Error;

use agentc_compiler::{
    runner::types::RunParams,
    toolchain::{errors::ToolchainError, traits::ErasedToolchain},
};

use crate::pipeline::{
    sender::Tx,
    steps::{cleanup::CleanupStepOutput, compile::CompileStepOutput},
    traits::Step,
};

#[derive(Debug, Error)]
pub enum LaunchStepError {
    #[error("toolchain error: {0}")]
    Toolchain(#[from] ToolchainError),

    #[error("event channel closed")]
    EventChannelClosed,
}

pub enum LaunchStepEvent {
    Launching,
    Exited { exit_code: Option<i32> },
}

pub struct LaunchStepInput {
    pub toolchain: Box<dyn ErasedToolchain>,
}

impl From<CleanupStepOutput<CompileStepOutput>> for LaunchStepInput {
    fn from(output: CleanupStepOutput<CompileStepOutput>) -> Self {
        LaunchStepInput { toolchain: output.inner.toolchain }
    }
}

pub struct LaunchStepOutput {
    pub exit_code: Option<i32>,
}

pub struct LaunchStep {
    context_dir: PathBuf,
    args: Vec<String>,
}

impl LaunchStep {
    pub fn new(context_dir: PathBuf, args: Vec<String>) -> Self {
        Self { context_dir, args }
    }
}

#[async_trait]
impl Step for LaunchStep {
    type Input = LaunchStepInput;
    type Output = LaunchStepOutput;
    type Event = LaunchStepEvent;
    type Error = LaunchStepError;

    async fn execute<S>(self, input: Self::Input, tx: S) -> Result<Self::Output, Self::Error>
    where
        S: Tx<Item = Self::Event> + Clone + Send + Sync + 'static,
    {
        // The invocation takes over the terminal, so the renderer must retire its
        // progress indicator before any child output appears.
        tx.send(LaunchStepEvent::Launching)
            .await
            .map_err(|_| LaunchStepError::EventChannelClosed)?;

        let outcome = input
            .toolchain
            .run_erased(RunParams::new(self.context_dir).with_args(self.args))
            .await?;

        tx.send(LaunchStepEvent::Exited { exit_code: outcome.exit_code })
            .await
            .map_err(|_| LaunchStepError::EventChannelClosed)?;

        Ok(LaunchStepOutput { exit_code: outcome.exit_code })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use tokio::sync::mpsc;

    use agentc_compiler::{
        compiler::{traits::OutputSink, types::CompileParams},
        runner::types::RunOutcome,
    };

    use super::*;

    type Recorder = Arc<Mutex<Vec<RunParams>>>;

    struct FakeToolchain {
        exit_code: Option<i32>,
        seen: Recorder,
    }

    impl FakeToolchain {
        fn new(exit_code: Option<i32>) -> Self {
            Self::recording(exit_code, Recorder::default())
        }

        fn recording(exit_code: Option<i32>, seen: Recorder) -> Self {
            Self { exit_code, seen }
        }
    }

    #[async_trait]
    impl ErasedToolchain for FakeToolchain {
        fn supports_run(&self) -> bool {
            true
        }

        async fn compile_erased(
            &mut self,
            _params: CompileParams,
            _output_sink: &dyn OutputSink,
        ) -> Result<(), ToolchainError> {
            Ok(())
        }

        async fn run_erased(&self, params: RunParams) -> Result<RunOutcome, ToolchainError> {
            self.seen
                .lock()
                .unwrap()
                .push(params);

            Ok(RunOutcome::new(self.exit_code))
        }
    }

    #[tokio::test]
    async fn the_invocation_status_is_reported() {
        let (tx, _rx) = mpsc::channel(8);

        assert_eq!(
            LaunchStep::new(PathBuf::from("/project"), Vec::new())
                .execute(
                    LaunchStepInput { toolchain: Box::new(FakeToolchain::new(Some(3))) },
                    tx,
                )
                .await
                .unwrap()
                .exit_code,
            Some(3),
        );
    }

    #[tokio::test]
    async fn an_invocation_killed_by_a_signal_reports_no_status() {
        let (tx, _rx) = mpsc::channel(8);

        assert_eq!(
            LaunchStep::new(PathBuf::from("/project"), Vec::new())
                .execute(
                    LaunchStepInput { toolchain: Box::new(FakeToolchain::new(None)) },
                    tx,
                )
                .await
                .unwrap()
                .exit_code,
            None,
        );
    }

    #[tokio::test]
    async fn the_context_directory_and_arguments_reach_the_runner_unchanged() {
        let (tx, _rx) = mpsc::channel(8);
        let seen = Recorder::default();

        LaunchStep::new(
            PathBuf::from("/project"),
            vec!["run".to_string(), "hello".to_string()],
        )
        .execute(
            LaunchStepInput {
                toolchain: Box::new(FakeToolchain::recording(Some(0), seen.clone())),
            },
            tx,
        )
        .await
        .unwrap();

        let seen = seen.lock().unwrap();

        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].context_dir, PathBuf::from("/project"));
        assert_eq!(seen[0].args, vec!["run", "hello"]);
    }
}
