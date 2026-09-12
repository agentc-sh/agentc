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
    A2a(ResolvedContextToolA2a),
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

    pub fn is_a2a(&self) -> bool {
        matches!(self, Self::A2a(_))
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
    /// Host program names registered as passthrough commands.
    pub commands: Vec<String>,

    /// The initial working directory inside the process filesystem.
    pub cwd: String,

    /// Environment variable forwarding policy.
    pub env: ResolvedContextToolBashEnv,

    /// Resource bounds applied to each execution.
    pub limits: ResolvedContextToolBashLimits,

    /// Whether shell state is shared across invocations.
    pub shared: bool,
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

/// Resolved configuration for a Python tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextToolPython {
    /// Absolute path to the directory containing the tool package's `pyproject.toml`.
    /// Resolved from the manifest-relative source path during the transform step.
    pub project_path: String,

    /// Absolute path to the `site-packages` directory inside the virtual environment
    /// created by the transform step. Embedded as a package tree bundle in the generated
    /// code so that all installed dependencies are compiled into the binary.
    pub site_packages_path: String,

    /// The importable Python module name for this tool package, derived from
    /// `[project].name` in `pyproject.toml` with hyphens replaced by underscores.
    /// Used as the executor entry module in the generated code.
    pub module_name: String,

    /// The name of the Python class that implements the tool interface.
    pub export_name: String,

    /// Which Python runtime backend to use. Defaults to `embedded` (RustPython).
    pub interpreter: ResolvedContextToolPythonInterpreter,
}

/// Selects the Python runtime backend for a Python tool.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedContextToolPythonInterpreter {
    /// Embed RustPython directly into the binary. Supports pure-Python packages only.
    #[default]
    Embedded,
    /// Link against a system CPython installation via PyO3. Supports C-extension packages,
    /// and requires a compatible CPython in the runtime environment.
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextToolA2a {
    pub url: RuntimeValue<String>,
    pub auth_token: Option<RuntimeValue<String>>,
    pub headers: HashMap<String, RuntimeValue<String>>,
    pub tenant: ResolvedContextToolA2aTenant,
    pub timeout_secs: Option<RuntimeValue<u64>>,
    pub default_accepted_output_modes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "policy", rename_all = "snake_case")]
pub enum ResolvedContextToolA2aTenant {
    Inherit,
    None,
    Fixed { id: RuntimeValue<String> },
}
