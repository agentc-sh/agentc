// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use rust_bash::{
    ExecutionLimits, NetworkPolicy as RustBashNetworkPolicy, OverlayFs, ReadWriteFs,
    RustBashBuilder,
};

use agentc_agent::{
    graph::state::GraphState,
    tools::{
        errors::ToolError,
        traits::TypedTool,
        types::{TypedToolInput, TypedToolOutput},
    },
    types::capability::CapabilitySet,
};

use crate::bash::{
    command::PassthroughCommand,
    config::{BashConfig, CommandPolicy, EnvPolicy, ExecLimits, FsPolicy, NetworkPolicy},
    errors::BashToolError,
};

/// Input to [`BashTool`].
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BashInput {
    /// The bash script or command string to execute inside the sandbox.
    pub command: String,
}

/// Output from [`BashTool`].
#[derive(Debug, Serialize, JsonSchema)]
pub struct BashOutput {
    /// Captured standard output from the script.
    pub stdout: String,
    /// Captured standard error from the script.
    pub stderr: String,
    /// Exit code returned by the script. `0` indicates success.
    pub exit_code: i32,
}

/// A tool that executes bash scripts inside a sandboxed interpreter.
///
/// Each invocation constructs a fresh interpreter instance from the shared
/// [`BashConfig`](crate::bash::config::BashConfig), offloads the synchronous
/// execution to a blocking thread via
/// [`tokio::task::spawn_blocking`](tokio::task::spawn_blocking), and returns
/// the captured stdout, stderr, and exit code.
///
/// Use [`BashToolBuilder`] to construct an instance.
pub struct BashTool {
    config: Arc<BashConfig>,
}

impl BashTool {
    pub fn builder() -> BashToolBuilder {
        BashToolBuilder::new()
    }
}

#[async_trait]
impl<S: GraphState + 'static> TypedTool<S> for BashTool {
    type Input = BashInput;
    type Output = BashOutput;
    type State = ();
    type StateUpdate = ();

    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        r#"Invoke bash commands and scripts."#
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet::from(["bash"])
    }

    async fn execute(
        &self,
        input: TypedToolInput<BashInput>,
    ) -> Result<TypedToolOutput<BashOutput, ()>, ToolError> {
        let config = self.config.clone();
        let args = input.args;

        tokio::task::spawn_blocking(move || {
            let env = config.env_policy.resolve();

            let mut builder = RustBashBuilder::new()
                .env(env)
                .cwd(config.cwd.clone())
                .execution_limits(ExecutionLimits {
                    max_execution_time: config.limits.max_execution_time,
                    max_output_size: config.limits.max_output_size,
                    max_command_count: config.limits.max_command_count,
                    max_loop_iterations: config.limits.max_loop_iterations,
                    ..Default::default()
                })
                .network_policy(RustBashNetworkPolicy {
                    enabled: config.network.enabled,
                    allowed_url_prefixes: config
                        .network
                        .allowed_url_prefixes
                        .clone(),
                    allowed_methods: config.network.allowed_methods.clone(),
                    max_redirects: config.network.max_redirects,
                    max_response_size: config.network.max_response_size,
                    timeout: config.network.timeout,
                });

            builder = match &config.fs_policy {
                FsPolicy::InMemory => builder,
                FsPolicy::Overlay(path) => {
                    builder.fs(Arc::new(OverlayFs::new(path).map_err(BashToolError::fs)?))
                }
                FsPolicy::ReadWrite(path) => {
                    builder.fs(Arc::new(ReadWriteFs::with_root(path).map_err(BashToolError::fs)?))
                }
            };

            if let CommandPolicy::Allow(names) = &config.command_policy {
                for name in names {
                    builder = builder.command(Arc::new(PassthroughCommand::new(name)));
                }
            }

            builder
                .build()
                .map_err(BashToolError::execution)?
                .exec(&args.command)
                .map(|r| BashOutput {
                    stdout: r.stdout,
                    stderr: r.stderr,
                    exit_code: r.exit_code,
                })
                .map_err(BashToolError::execution)
        })
        .await
        .map_err(|_| BashToolError::worker_panicked())?
        .map(TypedToolOutput::ok)
        .map_err(ToolError::from)
    }
}

/// Builder for [`BashTool`].
///
/// All configuration fields default to safe, restrictive values: in-memory
/// filesystem, no environment variables, conservative execution limits, and
/// no additional host commands.
pub struct BashToolBuilder {
    config: BashConfig,
}

impl BashToolBuilder {
    pub fn new() -> Self {
        Self { config: BashConfig::default() }
    }

    /// Set the command policy, controlling which additional host programs are
    /// available as passthrough commands inside the sandbox.
    pub fn command_policy(mut self, policy: CommandPolicy) -> Self {
        self.config.command_policy = policy;
        self
    }

    /// Set the filesystem policy, controlling which backend the interpreter
    /// uses for file operations.
    pub fn fs_policy(mut self, policy: FsPolicy) -> Self {
        self.config.fs_policy = policy;
        self
    }

    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.config.cwd = cwd.into();
        self
    }

    /// Set the environment policy, controlling which variables are forwarded
    /// to the interpreter.
    pub fn env_policy(mut self, policy: EnvPolicy) -> Self {
        self.config.env_policy = policy;
        self
    }

    /// Set the execution limits applied to each script run.
    pub fn limits(mut self, limits: ExecLimits) -> Self {
        self.config.limits = limits;
        self
    }

    /// Set the network policy for sandboxed `curl` invocations.
    pub fn network(mut self, network: NetworkPolicy) -> Self {
        self.config.network = network;
        self
    }

    pub fn build(self) -> BashTool {
        BashTool { config: Arc::new(self.config) }
    }
}

impl Default for BashToolBuilder {
    fn default() -> Self {
        Self::new()
    }
}
