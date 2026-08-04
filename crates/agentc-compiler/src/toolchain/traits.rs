// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;

use crate::{
    compiler::{
        traits::{Compiler, OutputSink},
        types::CompileParams,
    },
    runner::{
        traits::Runner,
        types::{RunOutcome, RunParams},
    },
    toolchain::errors::ToolchainError,
};

/// A compiler paired with the runner that invokes what that compiler produces.
pub trait Toolchain: Send + Sync {
    type Artifact: Send + Sync + 'static;

    /// The compiler for this toolchain, which produces an artifact of the associated type.
    fn compiler(&self) -> &dyn Compiler<Artifact = Self::Artifact>;

    /// The runner for this toolchain, if what it builds can be invoked.
    fn runner(&self) -> Option<&dyn Runner<Artifact = Self::Artifact>>;
}

/// A [`Toolchain`](crate::toolchain::traits::Toolchain) with its artifact type erased,
/// for use in dynamic dispatch.
#[async_trait]
pub trait ErasedToolchain: Send + Sync {
    /// Whether this toolchain can invoke what it builds.
    fn supports_run(&self) -> bool;

    async fn compile_erased(
        &mut self,
        params: CompileParams,
        output_sink: &dyn OutputSink,
    ) -> Result<(), ToolchainError>;

    async fn run_erased(&self, params: RunParams) -> Result<RunOutcome, ToolchainError>;
}

/// A [`Toolchain`](crate::toolchain::traits::Toolchain) holding the artifact it
/// produced.
pub struct ErasedToolchainCell<T>
where
    T: Toolchain,
{
    toolchain: T,
    artifact: Option<T::Artifact>,
}

impl<T> ErasedToolchainCell<T>
where
    T: Toolchain + 'static,
{
    pub fn new(toolchain: T) -> Self {
        Self { toolchain, artifact: None }
    }

    /// Erases a toolchain into an
    /// [`ErasedToolchain`](crate::toolchain::traits::ErasedToolchain).
    pub fn erase(toolchain: T) -> Box<dyn ErasedToolchain> {
        Box::new(Self::new(toolchain))
    }
}

#[async_trait]
impl<T> ErasedToolchain for ErasedToolchainCell<T>
where
    T: Toolchain + 'static,
{
    fn supports_run(&self) -> bool {
        self.toolchain.runner().is_some()
    }

    async fn compile_erased(
        &mut self,
        params: CompileParams,
        output_sink: &dyn OutputSink,
    ) -> Result<(), ToolchainError> {
        self.artifact = Some(
            self.toolchain
                .compiler()
                .compile(params, output_sink)
                .await?,
        );

        Ok(())
    }

    async fn run_erased(&self, params: RunParams) -> Result<RunOutcome, ToolchainError> {
        Ok(self
            .toolchain
            .runner()
            .ok_or(ToolchainError::RunUnsupported)?
            .run(
                self.artifact
                    .as_ref()
                    .ok_or(ToolchainError::NotBuilt)?,
                params,
            )
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::{
        compiler::{errors::CompilerError, traits::NullOutputSink},
        runner::errors::RunnerError,
    };

    struct MarkerArtifact {
        id: &'static str,
    }

    struct RecordingCompiler;

    #[async_trait]
    impl Compiler for RecordingCompiler {
        type Artifact = MarkerArtifact;

        async fn compile(
            &self,
            _params: CompileParams,
            _output_sink: &dyn OutputSink,
        ) -> Result<Self::Artifact, CompilerError> {
            Ok(MarkerArtifact { id: "built" })
        }
    }

    struct RecordingRunner {
        seen: Mutex<Vec<(&'static str, Vec<String>)>>,
    }

    #[async_trait]
    impl Runner for RecordingRunner {
        type Artifact = MarkerArtifact;

        async fn run(
            &self,
            artifact: &Self::Artifact,
            params: RunParams,
        ) -> Result<RunOutcome, RunnerError> {
            self.seen
                .lock()
                .unwrap()
                .push((artifact.id, params.args.clone()));

            Ok(RunOutcome::new(Some(0)))
        }
    }

    struct TestToolchain {
        compiler: RecordingCompiler,
        runner: Option<RecordingRunner>,
    }

    impl TestToolchain {
        fn with_runner() -> Self {
            Self {
                compiler: RecordingCompiler,
                runner: Some(RecordingRunner { seen: Mutex::new(Vec::new()) }),
            }
        }

        fn without_runner() -> Self {
            Self {
                compiler: RecordingCompiler,
                runner: None,
            }
        }
    }

    impl Toolchain for TestToolchain {
        type Artifact = MarkerArtifact;

        fn compiler(&self) -> &dyn Compiler<Artifact = Self::Artifact> {
            &self.compiler
        }

        fn runner(&self) -> Option<&dyn Runner<Artifact = Self::Artifact>> {
            self.runner
                .as_ref()
                .map(|runner| runner as &dyn Runner<Artifact = Self::Artifact>)
        }
    }

    #[test]
    fn supports_run_reflects_runner_presence() {
        assert!(ErasedToolchainCell::new(TestToolchain::with_runner()).supports_run());
        assert!(!ErasedToolchainCell::new(TestToolchain::without_runner()).supports_run());
    }

    #[tokio::test]
    async fn running_without_a_prior_compile_reports_not_built() {
        assert!(matches!(
            ErasedToolchainCell::new(TestToolchain::with_runner())
                .run_erased(RunParams::new("/project"))
                .await,
            Err(ToolchainError::NotBuilt),
        ));
    }

    #[tokio::test]
    async fn running_without_a_runner_reports_unsupported() {
        let mut cell = ErasedToolchainCell::new(TestToolchain::without_runner());

        cell.compile_erased(CompileParams::new("/project", "/out"), &NullOutputSink)
            .await
            .unwrap();

        assert!(matches!(
            cell.run_erased(RunParams::new("/project"))
                .await,
            Err(ToolchainError::RunUnsupported),
        ));
    }

    #[tokio::test]
    async fn running_passes_the_compiled_artifact_and_arguments_to_the_runner() {
        let mut cell = ErasedToolchainCell::new(TestToolchain::with_runner());

        cell.compile_erased(CompileParams::new("/project", "/out"), &NullOutputSink)
            .await
            .unwrap();
        cell.run_erased(RunParams::new("/project").with_args(["run", "hello"]))
            .await
            .unwrap();

        assert_eq!(
            cell.toolchain
                .runner
                .as_ref()
                .unwrap()
                .seen
                .lock()
                .unwrap()
                .as_slice(),
            [("built", vec!["run".to_string(), "hello".to_string()])],
        );
    }
}
