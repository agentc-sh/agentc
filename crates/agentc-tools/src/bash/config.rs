// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    time::Duration,
};

/// Controls which host programs are available as additional commands inside
/// the interpreter, on top of its built-in commands.
///
/// Each name in [`CommandPolicy::Allow`] is registered as a
/// [`PassthroughCommand`](crate::bash::command::PassthroughCommand) that
/// proxies invocations to the matching binary on the host system. Programs not
/// listed are not proxied; they are handled only by the interpreter's built-in
/// registry.
///
/// [`CommandPolicy::Unrestricted`] registers no additional programs. The full
/// set of built-in commands remains available regardless of which variant
/// is active.
#[derive(Debug, Clone, Default)]
pub enum CommandPolicy {
    /// No additional host programs are registered beyond the interpreter's
    /// built-in command set.
    #[default]
    Unrestricted,
    /// Each listed program name is registered as a passthrough to the real
    /// host binary of the same name.
    Allow(Vec<String>),
}

/// Controls which filesystem backend the interpreter uses.
///
/// See the `rust-bash` crate documentation for backend semantics.
#[derive(Debug, Clone, Default)]
pub enum FsPolicy {
    /// All filesystem operations are fully in-memory. No host files are
    /// read or written. This is the default.
    #[default]
    InMemory,
    /// Copy-on-write overlay over the given host directory. Reads come from
    /// disk; writes are kept in memory and never reach the host.
    Overlay(PathBuf),
    /// Direct passthrough to the host filesystem rooted at the given path.
    /// Reads and writes affect real files.
    ReadWrite(PathBuf),
}

/// Controls which environment variables are forwarded to the interpreter.
#[derive(Debug, Clone, Default)]
pub enum EnvPolicy {
    /// Inherit the full environment of the calling process.
    Inherit,
    /// Forward only the listed variable names.
    Allow(Vec<String>),
    /// Forward everything except the listed variable names.
    Deny(Vec<String>),
    /// Pass no environment variables.
    #[default]
    Empty,
}

impl EnvPolicy {
    pub fn resolve(&self) -> HashMap<String, String> {
        match self {
            Self::Inherit => std::env::vars().collect(),
            Self::Allow(keys) => std::env::vars()
                .filter(|(k, _)| keys.contains(k))
                .collect(),
            Self::Deny(keys) => std::env::vars()
                .filter(|(k, _)| !keys.contains(k))
                .collect(),
            Self::Empty => HashMap::new(),
        }
    }
}

/// Resource bounds applied to each script execution.
///
/// Maps directly to [`rust_bash::ExecutionLimits`](rust_bash::ExecutionLimits).
/// All fields default to conservative but practical values.
#[derive(Debug, Clone)]
pub struct ExecLimits {
    /// Maximum wall-clock time a script may run.
    pub max_execution_time: Duration,
    /// Maximum combined stdout and stderr output in bytes.
    pub max_output_size: usize,
    /// Maximum number of commands that may be dispatched.
    pub max_command_count: usize,
    /// Maximum number of loop iterations across all loops.
    pub max_loop_iterations: usize,
}

impl Default for ExecLimits {
    fn default() -> Self {
        Self {
            max_execution_time: Duration::from_secs(30),
            max_output_size: 10 * 1024 * 1024,
            max_command_count: 10_000,
            max_loop_iterations: 10_000,
        }
    }
}

/// Network access policy forwarded to the interpreter's sandboxed `curl`.
///
/// URL matching uses prefix comparison; wildcards are not supported.
#[derive(Debug, Clone, Default)]
pub struct NetworkPolicy {
    /// Whether network access via `curl` is permitted at all.
    pub enabled: bool,
    /// URL prefixes that `curl` is allowed to contact. Only used when
    /// `enabled` is `true`. An empty list blocks all requests even if
    /// `enabled` is `true`.
    pub allowed_url_prefixes: Vec<String>,
    /// HTTP methods that `curl` is allowed to use. Only used when `enabled`
    /// is `true`. An empty list blocks all requests even if `enabled` is `true`.
    pub allowed_methods: HashSet<String>,
    /// Maximum number of redirects `curl` may follow before aborting the request.
    pub max_redirects: usize,
    /// Maximum size of the response body `curl` may receive before aborting the request.
    pub max_response_size: usize,
    /// Maximum duration of a `curl` request before it is aborted.
    pub timeout: Duration,
}

/// Full configuration for a [`BashTool`](crate::bash::tool::BashTool) instance.
#[derive(Debug, Clone, Default)]
pub struct BashConfig {
    /// Which additional host programs to register as passthrough commands.
    pub command_policy: CommandPolicy,
    /// Which filesystem backend the interpreter operates against.
    pub fs_policy: FsPolicy,
    /// Which environment variables are forwarded to the interpreter.
    pub env_policy: EnvPolicy,
    /// Resource bounds applied to each execution.
    pub limits: ExecLimits,
    /// Network access policy for sandboxed `curl` invocations.
    pub network: NetworkPolicy,
    /// The CWD inside the sandbox for each command execution.
    pub cwd: String,
}
