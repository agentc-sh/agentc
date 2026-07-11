// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use sanitizer::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use validator::{Validate, ValidateArgs, ValidationErrors};

use agentc_blocks::types::RuntimeValue;
use agentc_compiler::asset::types::{AssetOrigin, AssetRef};

/// A tool definition as declared in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestTool {
    /// Human-readable description of what the tool does.
    #[serde(default)]
    #[sanitizer(trim)]
    pub description: Option<String>,

    /// Whether this tool is enabled.
    #[serde(default = "default_tool_enabled")]
    pub enabled: RuntimeValue<bool>,

    /// Capabilities required to invoke this tool. Always baked in at
    /// compile time.
    #[serde(default)]
    pub capabilities: Vec<String>,

    /// Optional flat config values, each of which may be a constant or
    /// a runtime environment variable.
    #[serde(default)]
    pub config: HashMap<String, RuntimeValue<String>>,

    /// The type-specific fields for this tool.
    #[serde(flatten)]
    #[validate(nested)]
    pub kind: ManifestToolKind,
}

fn default_tool_enabled() -> RuntimeValue<bool> {
    RuntimeValue::constant(true)
}

impl ManifestTool {
    /// Collect all source URIs from this tool into the given vector,
    /// attributed to the tool with the given name.
    pub fn collect_assets(&self, name: &str, assets: &mut Vec<AssetRef>) {
        match &self.kind {
            ManifestToolKind::Javascript(javascript) => {
                assets.push(AssetRef::new(javascript.source.clone(), AssetOrigin::tool(name)));
            }
            ManifestToolKind::Python(python) => {
                assets.push(AssetRef::new(python.source.clone(), AssetOrigin::tool(name)));
            }
            _ => {}
        }
    }
}

/// The type-specific fields for each tool variant.
#[derive(Debug, Clone, Serialize, Deserialize, Sanitizer)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ManifestToolKind {
    Javascript(ManifestJavascriptTool),
    Mcp(ManifestMcpTool),
    Bash(ManifestBashTool),
    Python(ManifestPythonTool),
    A2a(ManifestA2aTool),
}

impl<'v_a> ValidateArgs<'v_a> for ManifestToolKind {
    type Args = ();

    fn validate_with_args(&self, args: Self::Args) -> Result<(), ValidationErrors> {
        match self {
            ManifestToolKind::Javascript(value) => value.validate_with_args(args),
            ManifestToolKind::Mcp(value) => value.validate_with_args(args),
            ManifestToolKind::Bash(value) => value.validate_with_args(args),
            ManifestToolKind::Python(value) => value.validate_with_args(args),
            ManifestToolKind::A2a(value) => value.validate_with_args(args),
        }
    }
}

impl Validate for ManifestToolKind {
    fn validate(&self) -> Result<(), ValidationErrors> {
        self.validate_with_args(())
    }
}

/// Fields specific to a Python package tool.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestPythonTool {
    /// Path to the directory containing `pyproject.toml`, relative to the manifest file.
    ///
    /// The transform step runs `uv sync` in this directory to install dependencies and
    /// create a virtual environment. Both the project source and the installed
    /// `site-packages` are embedded into the binary at compile time.
    #[validate(length(min = 1))]
    #[sanitizer(trim)]
    pub source: String,

    /// Which Python runtime backend to use. Defaults to `embedded` (RustPython).
    ///
    /// `embedded` supports pure-Python packages only. `static` (CPython via PyO3)
    /// supports C-extension packages but is not yet implemented.
    #[serde(default)]
    pub interpreter: ManifestPythonInterpreter,
}

/// Selects the Python runtime backend for a Python tool.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestPythonInterpreter {
    /// Embed RustPython directly into the binary. Supports pure-Python packages only.
    #[default]
    Embedded,
    /// Link against a system CPython installation via PyO3. Supports C-extension packages.
    /// Not yet implemented; accepted in the manifest but generates no code.
    Static,
}

/// Fields specific to a JavaScript file tool.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestJavascriptTool {
    /// Path to the package directory containing the tool's code, relative to the manifest file.
    #[validate(length(min = 1))]
    #[sanitizer(trim)]
    pub source: String,

    /// Name of the JS export object for this tool.
    ///
    /// When absent the manifest tool block name is used as the export name as-is.
    #[serde(default)]
    #[sanitizer(trim)]
    pub export: Option<String>,
}

/// Transport-specific configuration for an MCP server tool.
///
/// The `transport` field selects which variant is active. `stdio` spawns a local subprocess;
/// `http` connects to a remote server over streamable HTTP.
#[derive(Debug, Clone, Serialize, Deserialize, Sanitizer)]
#[serde(tag = "transport", rename_all = "snake_case")]
pub enum ManifestMcpTool {
    Stdio {
        /// Command used to spawn the MCP server subprocess.
        command: RuntimeValue<String>,
        /// Arguments passed to the command.
        #[serde(default)]
        args: Vec<RuntimeValue<String>>,
        /// Environment variables forwarded to the subprocess.
        #[serde(default)]
        config: HashMap<String, RuntimeValue<String>>,
    },
    Http {
        /// Base URL of the MCP server.
        url: RuntimeValue<String>,
        /// Optional bearer token sent in the `Authorization` header.
        #[serde(default)]
        auth_token: Option<RuntimeValue<String>>,
        /// Additional HTTP headers sent with every request.
        #[serde(default)]
        headers: HashMap<String, RuntimeValue<String>>,
    },
}

impl<'v_a> ValidateArgs<'v_a> for ManifestMcpTool {
    type Args = ();

    fn validate_with_args(&self, _args: Self::Args) -> Result<(), ValidationErrors> {
        Ok(())
    }
}

impl Validate for ManifestMcpTool {
    fn validate(&self) -> Result<(), ValidationErrors> {
        self.validate_with_args(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestA2aTool {
    pub url: RuntimeValue<String>,

    #[serde(default)]
    pub auth_token: Option<RuntimeValue<String>>,

    #[serde(default)]
    pub headers: HashMap<String, RuntimeValue<String>>,

    #[serde(default)]
    pub tenant: ManifestA2aTenant,

    #[serde(default)]
    pub timeout_secs: Option<RuntimeValue<u64>>,

    #[serde(default)]
    pub default_accepted_output_modes: Vec<String>,
}

impl<'v_a> ValidateArgs<'v_a> for ManifestA2aTool {
    type Args = ();

    fn validate_with_args(&self, _args: Self::Args) -> Result<(), ValidationErrors> {
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "policy", rename_all = "snake_case")]
pub enum ManifestA2aTenant {
    #[default]
    Inherit,
    None,
    Fixed {
        id: RuntimeValue<String>,
    },
}

/// Fields specific to a bash sandbox tool.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestBashTool {
    /// Additional host programs to register as passthrough commands.
    ///
    /// Each name is proxied to the real binary on the host. When absent,
    /// no extra programs are registered beyond the interpreter's built-in set.
    #[serde(default)]
    pub commands: Vec<String>,

    /// Filesystem backend configuration.
    #[serde(default)]
    pub fs: ManifestBashFs,

    /// Environment variable forwarding policy.
    #[serde(default)]
    pub env: ManifestBashEnv,

    /// Resource limits applied to each execution.
    #[serde(default)]
    pub limits: ManifestBashLimits,

    /// Network access policy for sandboxed `curl` invocations.
    #[serde(default)]
    pub network: ManifestBashNetwork,
}

/// Filesystem backend configuration for a bash sandbox tool.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestBashFs {
    /// Backend kind. Defaults to `"in_memory"`.
    ///
    /// Valid values: `"in_memory"`, `"overlay"`, `"read_write"`.
    /// `"overlay"` and `"read_write"` require `path` to be set.
    #[serde(default)]
    pub kind: ManifestBashFsKind,

    /// Host path used by `"overlay"` and `"read_write"` backends.
    #[serde(default)]
    pub path: Option<String>,

    /// The current working directory inside the sandbox. Defaults to `"/home/agent"`.
    #[serde(default = "default_bash_cwd")]
    pub cwd: String,
}

fn default_bash_cwd() -> String {
    "/home/agent".to_string()
}

/// Filesystem backend kind for a bash sandbox tool.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestBashFsKind {
    #[default]
    InMemory,
    Overlay,
    ReadWrite,
}

/// Environment variable forwarding policy for a bash sandbox tool.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestBashEnv {
    /// Forwarding policy kind. Defaults to `"empty"`.
    ///
    /// Valid values: `"empty"`, `"inherit"`, `"allow"`, `"deny"`.
    #[serde(default)]
    pub kind: ManifestBashEnvKind,

    /// Variable names used by `"allow"` and `"deny"` policies.
    #[serde(default)]
    pub vars: Vec<String>,
}

/// Environment variable forwarding kind for a bash sandbox tool.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestBashEnvKind {
    /// Pass no environment variables.
    #[default]
    Empty,
    /// Inherit the full environment of the calling process.
    Inherit,
    /// Forward only the variables listed in `vars`.
    Allow,
    /// Forward everything except the variables listed in `vars`.
    Deny,
}

/// Resource limits for a bash sandbox tool. All fields are optional and fall
/// back to the interpreter's built-in defaults when absent.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestBashLimits {
    /// Maximum wall-clock execution time in seconds. Defaults to 30.
    #[serde(default)]
    pub max_execution_time_secs: Option<u64>,
    /// Maximum combined output size in bytes. Defaults to 10 MiB.
    #[serde(default)]
    pub max_output_size: Option<usize>,
    /// Maximum number of commands that may be dispatched. Defaults to 10 000.
    #[serde(default)]
    pub max_command_count: Option<usize>,
    /// Maximum number of loop iterations. Defaults to 10 000.
    #[serde(default)]
    pub max_loop_iterations: Option<usize>,
}

/// Network access policy for sandboxed `curl` in a bash sandbox tool.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestBashNetwork {
    /// Whether sandboxed `curl` network access is enabled. Defaults to false.
    #[serde(default)]
    pub enabled: Option<bool>,
    /// URL prefixes that sandboxed `curl` may contact.
    #[serde(default)]
    pub allowed_url_prefixes: Vec<String>,
    /// HTTP methods that sandboxed `curl` may use.
    #[serde(default)]
    pub allowed_methods: HashSet<String>,
    /// Maximum redirects `curl` may follow. Defaults to 0.
    #[serde(default)]
    pub max_redirects: Option<usize>,
    /// Maximum response body size in bytes. Defaults to 10 MiB.
    #[serde(default)]
    pub max_response_size: Option<usize>,
    /// Maximum duration of a `curl` request in seconds. Defaults to 30.
    #[serde(default)]
    pub network_timeout_secs: Option<u64>,
}
