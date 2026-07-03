// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::RuntimeValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextTool {
    /// The tool name exactly as declared in the manifest (e.g. `"adder"`).
    pub name: String,

    /// Human-readable description.
    pub description: Option<String>,

    /// Whether the tool is active. May be a runtime env-var check or a compile-time constant.
    pub enabled: RuntimeValue<bool>,

    /// Capability strings baked in at compile time (e.g. `["network"]`).
    pub capabilities: Vec<String>,

    /// Optional flat config values forwarded from the manifest, each of which
    /// may be a compile-time constant or a runtime env-var lookup.
    pub config: HashMap<String, RuntimeValue<String>>,

    /// Kind-specific resolved data.
    pub kind: ResolvedContextToolKind,
}

/// Discriminates between tool implementation strategies.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResolvedContextToolKind {
    Javascript(ResolvedContextToolJavascript),
    Mcp(ResolvedContextToolMcp),
    Bash(ResolvedContextToolBash),
    Python(ResolvedContextToolPython),
}

impl ResolvedContextToolKind {
    pub fn is_javascript(&self) -> bool {
        matches!(self, Self::Javascript(_))
    }

    pub fn is_mcp(&self) -> bool {
        matches!(self, Self::Mcp(_))
    }

    pub fn is_bash(&self) -> bool {
        matches!(self, Self::Bash(_))
    }

    pub fn is_python(&self) -> bool {
        matches!(self, Self::Python(_))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextToolJavascript {
    /// Absolute path to the bundled `.js` file in the artifact store.
    /// Used directly in the generated `include_str!("...")` call.
    pub bundle_path: String,

    /// Name of the JS export object to invoke when the tool is called.
    pub export_name: String,
}

/// Resolved configuration for an MCP server tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextToolMcp {
    pub transport: ResolvedContextToolMcpTransport,
}

/// Resolved configuration for a bash sandbox tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextToolBash {
    /// Host program names registered as passthrough commands. Empty means no
    /// additional programs beyond the interpreter's built-in set.
    pub commands: Vec<String>,
    /// Filesystem backend policy.
    pub fs: ResolvedContextToolBashFs,
    /// Environment variable forwarding policy.
    pub env: ResolvedContextToolBashEnv,
    /// Resource bounds applied to each execution.
    pub limits: ResolvedContextToolBashLimits,
    /// Network access policy for sandboxed `curl` invocations.
    pub network: ResolvedContextToolBashNetwork,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextToolBashFs {
    /// The kind of filesystem backend to use.
    pub kind: ResolvedContextToolBashFsKind,
    /// The CWD inside the sandbox for each command execution.
    pub cwd: String,
}

/// Filesystem backend policy for a bash sandbox tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedContextToolBashFsKind {
    /// All file operations are fully in-memory.
    InMemory,
    /// Copy-on-write overlay over the given host path.
    Overlay(String),
    /// Direct passthrough to the host filesystem at the given path.
    ReadWrite(String),
}

/// Environment variable forwarding policy for a bash sandbox tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedContextToolBashEnv {
    /// Pass no environment variables.
    Empty,
    /// Inherit the full environment of the calling process.
    Inherit,
    /// Forward only the listed variable names.
    Allow(Vec<String>),
    /// Forward everything except the listed variable names.
    Deny(Vec<String>),
}

/// Resource bounds for a bash sandbox tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextToolBashLimits {
    /// Maximum wall-clock execution time in seconds.
    pub max_execution_time_secs: u64,
    /// Maximum combined stdout and stderr output in bytes.
    pub max_output_size: usize,
    /// Maximum number of commands that may be dispatched.
    pub max_command_count: usize,
    /// Maximum number of loop iterations across all loops.
    pub max_loop_iterations: usize,
}

/// Network access policy for sandboxed `curl` in a bash sandbox tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextToolBashNetwork {
    /// Whether sandboxed `curl` network access is permitted.
    pub enabled: bool,
    /// URL prefixes `curl` is allowed to contact.
    pub allowed_url_prefixes: Vec<String>,
    /// HTTP methods `curl` is allowed to use.
    pub allowed_methods: Vec<String>,
    /// Maximum redirects `curl` may follow.
    pub max_redirects: usize,
    /// Maximum response body size in bytes that `curl` may receive.
    pub max_response_size: usize,
    /// Maximum duration of a `curl` request in seconds.
    pub network_timeout_secs: u64,
}

/// Resolved configuration for a Python tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextToolPython {
    /// Absolute path to the directory containing the tool package's `pyproject.toml`.
    /// Resolved from the manifest-relative source path during the transform step.
    pub project_path: String,

    /// Absolute path to the `site-packages` directory inside the virtual environment
    /// created by the transform step. Passed directly to `py_freeze!` in the generated
    /// code so that all installed dependencies are embedded at compile time.
    pub site_packages_path: String,

    /// The importable Python module name for this tool package, derived from
    /// `[project].name` in `pyproject.toml` with hyphens replaced by underscores.
    /// Passed to `PythonToolBuilder::module` in the generated code so the runtime
    /// imports the package before looking up the tool in `__tool_registry__`.
    pub module_name: String,

    /// Which Python runtime backend to use. Defaults to `embedded` (RustPython).
    /// `static` (PyO3/CPython) is accepted and round-trips through the resolved context
    /// but generates no code until the PyO3 backend is added in a future MR.
    pub interpreter: ResolvedContextToolPythonInterpreter,
}

/// Selects the Python runtime backend for a Python tool.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedContextToolPythonInterpreter {
    /// Embed RustPython directly into the binary. Supports pure-Python packages only.
    #[default]
    Embedded,
    /// Link against a system CPython installation via PyO3. Supports C-extension packages.
    /// Not yet implemented; accepted in the manifest but no code is generated.
    Static,
}

/// Transport-specific resolved configuration for an MCP server tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "transport", rename_all = "snake_case")]
pub enum ResolvedContextToolMcpTransport {
    Stdio {
        command: RuntimeValue<String>,
        args: Vec<RuntimeValue<String>>,
        env: HashMap<String, RuntimeValue<String>>,
    },
    Http {
        url: RuntimeValue<String>,
        auth_token: Option<RuntimeValue<String>>,
        headers: HashMap<String, RuntimeValue<String>>,
    },
}
