// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::Mutex;

use agentc_agent::{
    graph::state::GraphState,
    tools::{
        errors::ToolError,
        traits::TypedTool,
        types::{TypedToolInput, TypedToolOutput},
    },
    types::capability::CapabilitySet,
};

use crate::{registry::SkillRegistry, skill::AllowedTool};

/// Controls how unknown or binary skill resources are written to disk for execution.
///
/// Interpreted languages (Bash, Python, Node.js, TypeScript, Ruby) are always
/// executed via stdin regardless of this policy; only resources that cannot be
/// piped to an interpreter are affected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializationPolicy {
    /// Write to a restricted temp file per invocation; deleted after the process exits.
    OnDemand,
    /// Write once on first invocation per script and reuse for the lifetime of the tool.
    Eager,
}

/// Detected interpreter for a skill script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptKind {
    Bash,
    Python,
    NodeCommonJs,
    NodeModule,
    TypeScript,
    Ruby,
    /// No recognized interpreter; must be materialized to disk before execution.
    Unknown,
}

impl ScriptKind {
    /// Detect the script kind from its path extension, falling back to shebang
    /// parsing when content is available.
    fn detect(path: &str, content: &str) -> Self {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match ext {
            "sh" | "bash" => return Self::Bash,
            "py" => return Self::Python,
            "js" | "cjs" => return Self::NodeCommonJs,
            "mjs" => return Self::NodeModule,
            "ts" | "mts" | "cts" => return Self::TypeScript,
            "rb" => return Self::Ruby,
            _ => {}
        }

        // Fall back to shebang detection.
        if let Some(first_line) = content.lines().next()
            && let Some(interp) = first_line.strip_prefix("#!")
        {
            let interp = interp.trim();

            if interp.contains("bash") || interp.contains("/sh") {
                return Self::Bash;
            }
            if interp.contains("python") {
                return Self::Python;
            }
            if interp.contains("tsx") || interp.contains("ts-node") {
                return Self::TypeScript;
            }
            // Deno and Bun both support TypeScript natively.
            if interp.contains("deno") || interp.contains("bun") {
                return Self::TypeScript;
            }
            if interp.contains("node") {
                return Self::NodeCommonJs;
            }
            if interp.contains("ruby") {
                return Self::Ruby;
            }
        }

        Self::Unknown
    }

    /// Returns the interpreter program and its leading args for stdin execution,
    /// or `None` when the script must be materialized to disk instead.
    fn stdin_invocation(&self) -> Option<(&'static str, &'static [&'static str])> {
        match self {
            Self::Bash => Some(("bash", &["-s"])),
            Self::Python => Some(("python3", &["-"])),
            Self::NodeCommonJs => Some(("node", &["--input-type=commonjs"])),
            Self::NodeModule => Some(("node", &["--input-type=module"])),
            Self::TypeScript => Some(("tsx", &["-"])),
            Self::Ruby => Some(("ruby", &["-"])),
            Self::Unknown => None,
        }
    }

    /// Returns the interpreter program and its leading args for running a
    /// script from a filesystem path.
    fn path_invocation(&self, script_path: &Path) -> (String, Vec<String>) {
        let p = script_path
            .to_string_lossy()
            .into_owned();
        match self {
            Self::Bash => ("bash".to_string(), vec![p]),
            Self::Python => ("python3".to_string(), vec![p]),
            Self::NodeCommonJs | Self::NodeModule => ("node".to_string(), vec![p]),
            Self::TypeScript => ("tsx".to_string(), vec![p]),
            Self::Ruby => ("ruby".to_string(), vec![p]),
            // Exec directly; the OS handles the shebang.
            Self::Unknown => (p, vec![]),
        }
    }
}

/// Shared output type for all script execution strategies.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RunSkillScriptOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Handles low-level process invocation for skill scripts.
///
/// Each method corresponds to one execution strategy. [`RunSkillScriptTool`]
/// selects the appropriate strategy based on the [`ScriptKind`] and the active
/// [`MaterializationPolicy`].
struct ScriptRunner;

impl ScriptRunner {
    /// Execute a script by piping its content to an interpreter via stdin.
    ///
    /// The script content is never written to disk.
    async fn run_via_stdin(
        &self,
        program: &str,
        prog_args: &[&str],
        content: &str,
        user_args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<RunSkillScriptOutput, ToolError> {
        let mut child = Command::new(program)
            .args(prog_args)
            .args(user_args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .envs(env)
            .spawn()
            .map_err(|e| {
                ToolError::execution_error(
                    "run_skill_script",
                    format!("failed to spawn '{}': {}", program, e),
                )
            })?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(content.as_bytes())
                .await
                .map_err(|e| {
                    ToolError::execution_error(
                        "run_skill_script",
                        format!("failed to write stdin: {}", e),
                    )
                })?;
            // Dropping stdin closes the pipe and signals EOF to the process.
        }

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| {
                ToolError::execution_error(
                    "run_skill_script",
                    format!("failed to wait for process: {}", e),
                )
            })?;

        Ok(RunSkillScriptOutput {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    /// Execute a script from a path already present on the filesystem.
    async fn run_from_path(
        &self,
        program: &str,
        prog_args: &[String],
        user_args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<RunSkillScriptOutput, ToolError> {
        let output = Command::new(program)
            .args(prog_args)
            .args(user_args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .envs(env)
            .output()
            .await
            .map_err(|e| {
                ToolError::execution_error(
                    "run_skill_script",
                    format!("failed to spawn '{}': {}", program, e),
                )
            })?;

        Ok(RunSkillScriptOutput {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    /// Write `content` to `path` with owner-execute-only permissions.
    async fn write_executable(&self, path: &Path, content: &[u8]) -> Result<(), String> {
        #[cfg(unix)]
        {
            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o700)
                .open(path)
                .await
                .map_err(|e| format!("failed to create temp file: {}", e))?;

            file.write_all(content)
                .await
                .map_err(|e| format!("failed to write temp file: {}", e))?;
        }

        #[cfg(not(unix))]
        tokio::fs::write(path, content)
            .await
            .map_err(|e| format!("failed to write temp file: {}", e))?;

        Ok(())
    }
}

/// Input to the [`RunSkillScriptTool`].
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RunSkillScriptInput {
    /// Name of the skill containing the script.
    pub skill_name: String,
    /// Relative path to the script within the skill, e.g. `scripts/extract.py`.
    pub script_path: String,
    /// Arguments passed to the script after the interpreter args.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables to set for the script process.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// A tool for executing scripts bundled with a skill.
///
/// Interpreted scripts (Bash, Python, Node.js, TypeScript, Ruby) are piped
/// directly to the interpreter via stdin and are never written to disk.
/// Resources of unrecognized types are governed by [`MaterializationPolicy`].
pub struct RunSkillScriptTool {
    registry: Arc<SkillRegistry>,
    policy: MaterializationPolicy,
    runner: ScriptRunner,
    /// Temp directory root for [`MaterializationPolicy::Eager`] scripts.
    ///
    /// `None` when policy is [`MaterializationPolicy::OnDemand`].
    temp_root: Option<PathBuf>,
    /// Cache mapping `(skill_name, script_path)` to materialized temp paths.
    ///
    /// Only used when policy is [`MaterializationPolicy::Eager`].
    cache: Mutex<HashMap<(String, String), PathBuf>>,
}

impl RunSkillScriptTool {
    /// Create a new tool with the given registry and materialization policy.
    ///
    /// For [`MaterializationPolicy::Eager`] a shared temp directory is created
    /// immediately. It is removed when the tool is dropped.
    pub fn new(registry: Arc<SkillRegistry>, policy: MaterializationPolicy) -> Self {
        let temp_root = match policy {
            MaterializationPolicy::Eager => {
                let path =
                    std::env::temp_dir().join(format!("agentc-skills-{}", uuid::Uuid::new_v4()));
                std::fs::create_dir_all(&path).ok();
                Some(path)
            }
            MaterializationPolicy::OnDemand => None,
        };

        Self {
            registry,
            policy,
            runner: ScriptRunner,
            temp_root,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Materialize binary content to a temp file and execute it via [`ScriptRunner`].
    ///
    /// Under [`MaterializationPolicy::OnDemand`] the file is deleted after the
    /// process exits. Under [`MaterializationPolicy::Eager`] the path is cached
    /// and reused for subsequent invocations.
    async fn run_materialized(
        &self,
        skill_name: &str,
        script_path: &str,
        content: &str,
        kind: &ScriptKind,
        user_args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<RunSkillScriptOutput, ToolError> {
        let file_name = Path::new(script_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        let exec_path = match self.policy {
            MaterializationPolicy::Eager => {
                let key = (skill_name.to_string(), script_path.to_string());
                let mut cache = self.cache.lock().await;

                if let Some(path) = cache.get(&key) {
                    path.clone()
                } else {
                    let path = self
                        .temp_root
                        .as_ref()
                        .unwrap()
                        .join(format!("{}-{}", uuid::Uuid::new_v4(), file_name));

                    self.runner
                        .write_executable(&path, content.as_bytes())
                        .await
                        .map_err(|e| ToolError::execution_error("run_skill_script", e))?;

                    cache.insert(key, path.clone());
                    path
                }
            }

            MaterializationPolicy::OnDemand => {
                let path =
                    std::env::temp_dir().join(format!("{}-{}", uuid::Uuid::new_v4(), file_name));

                self.runner
                    .write_executable(&path, content.as_bytes())
                    .await
                    .map_err(|e| ToolError::execution_error("run_skill_script", e))?;

                path
            }
        };

        let (program, prog_args) = kind.path_invocation(&exec_path);
        let result = self
            .runner
            .run_from_path(&program, &prog_args, user_args, env)
            .await;

        if self.policy == MaterializationPolicy::OnDemand {
            let _ = tokio::fs::remove_file(&exec_path).await;
        }

        result
    }
}

impl Drop for RunSkillScriptTool {
    fn drop(&mut self) {
        if let Some(ref dir) = self.temp_root {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

#[async_trait]
impl<S: GraphState + 'static> TypedTool<S> for RunSkillScriptTool {
    type Input = RunSkillScriptInput;
    type Output = RunSkillScriptOutput;
    type State = ();
    type StateUpdate = ();

    fn name(&self) -> &str {
        "run_skill_script"
    }

    fn description(&self) -> &str {
        r#"Execute a script bundled with a skill. Interpreted scripts are piped
        directly to the interpreter via stdin and never written to disk. Use
        get_skill first to discover the scripts available within a skill."#
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet::from(["skills::execute"])
    }

    async fn execute(
        &self,
        input: TypedToolInput<RunSkillScriptInput>,
    ) -> Result<TypedToolOutput<RunSkillScriptOutput, ()>, ToolError> {
        let skill = self
            .registry
            .get(&input.args.skill_name)
            .ok_or_else(|| {
                ToolError::not_found(format!("skill '{}' not found", input.args.skill_name))
            })?;

        // Enforce allowed-tools: if the skill declares a tool allowlist, Bash
        // must be present to permit any script execution via this tool.
        if !skill.allowed_tools.is_empty()
            && !skill
                .allowed_tools
                .iter()
                .filter_map(|t| AllowedTool::parse(t))
                .any(|t| t.permits("Bash"))
        {
            return Err(ToolError::execution_error(
                "run_skill_script",
                format!(
                    "skill '{}' does not permit script execution in its allowed-tools declaration",
                    input.args.skill_name,
                ),
            ));
        }

        let content = skill
            .read_resource(&input.args.script_path)
            .await
            .map_err(|e| ToolError::execution_error("run_skill_script", e.to_string()))?;

        let kind = ScriptKind::detect(&input.args.script_path, &content);

        let output = if let Some((program, prog_args)) = kind.stdin_invocation() {
            self.runner
                .run_via_stdin(program, prog_args, &content, &input.args.args, &input.args.env)
                .await?
        } else {
            self.run_materialized(
                &input.args.skill_name,
                &input.args.script_path,
                &content,
                &kind,
                &input.args.args,
                &input.args.env,
            )
            .await?
        };

        Ok(TypedToolOutput::ok(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // ScriptKind::detect -- extension-based
    // -------------------------------------------------------------------------

    #[test]
    fn detect_bash_extensions() {
        assert_eq!(ScriptKind::detect("run.sh", ""), ScriptKind::Bash);
        assert_eq!(ScriptKind::detect("run.bash", ""), ScriptKind::Bash);
    }

    #[test]
    fn detect_python_extension() {
        assert_eq!(ScriptKind::detect("process.py", ""), ScriptKind::Python);
    }

    #[test]
    fn detect_node_extensions() {
        assert_eq!(ScriptKind::detect("index.js", ""), ScriptKind::NodeCommonJs);
        assert_eq!(ScriptKind::detect("index.cjs", ""), ScriptKind::NodeCommonJs);
        assert_eq!(ScriptKind::detect("index.mjs", ""), ScriptKind::NodeModule);
    }

    #[test]
    fn detect_typescript_extensions() {
        assert_eq!(ScriptKind::detect("index.ts", ""), ScriptKind::TypeScript);
        assert_eq!(ScriptKind::detect("index.mts", ""), ScriptKind::TypeScript);
        assert_eq!(ScriptKind::detect("index.cts", ""), ScriptKind::TypeScript);
    }

    #[test]
    fn detect_ruby_extension() {
        assert_eq!(ScriptKind::detect("script.rb", ""), ScriptKind::Ruby);
    }

    // -------------------------------------------------------------------------
    // ScriptKind::detect -- shebang fallback
    // -------------------------------------------------------------------------

    #[test]
    fn detect_bash_shebang() {
        assert_eq!(ScriptKind::detect("script", "#!/bin/bash\necho hi"), ScriptKind::Bash);
        assert_eq!(ScriptKind::detect("script", "#!/usr/bin/env bash\necho hi"), ScriptKind::Bash);
        assert_eq!(ScriptKind::detect("script", "#!/bin/sh\necho hi"), ScriptKind::Bash);
    }

    #[test]
    fn detect_python_shebang() {
        assert_eq!(
            ScriptKind::detect("script", "#!/usr/bin/env python3\nprint('hi')"),
            ScriptKind::Python
        );
        assert_eq!(ScriptKind::detect("script", "#!/usr/bin/python\n"), ScriptKind::Python);
    }

    #[test]
    fn detect_node_shebang() {
        assert_eq!(ScriptKind::detect("script", "#!/usr/bin/env node\n"), ScriptKind::NodeCommonJs);
    }

    #[test]
    fn detect_tsx_shebang() {
        assert_eq!(ScriptKind::detect("script", "#!/usr/bin/env tsx\n"), ScriptKind::TypeScript);
        assert_eq!(
            ScriptKind::detect("script", "#!/usr/bin/env ts-node\n"),
            ScriptKind::TypeScript
        );
    }

    #[test]
    fn detect_deno_and_bun_shebang_map_to_typescript() {
        assert_eq!(ScriptKind::detect("script", "#!/usr/bin/env deno\n"), ScriptKind::TypeScript);
        assert_eq!(ScriptKind::detect("script", "#!/usr/bin/env bun\n"), ScriptKind::TypeScript);
    }

    #[test]
    fn detect_ruby_shebang() {
        assert_eq!(ScriptKind::detect("script", "#!/usr/bin/env ruby\n"), ScriptKind::Ruby);
    }

    #[test]
    fn detect_unknown_no_extension_no_shebang() {
        assert_eq!(ScriptKind::detect("binary", ""), ScriptKind::Unknown);
        assert_eq!(ScriptKind::detect("binary", "ELF..."), ScriptKind::Unknown);
    }

    #[test]
    fn detect_extension_takes_precedence_over_shebang() {
        // .py extension wins even if there is a bash shebang in the content.
        assert_eq!(ScriptKind::detect("script.py", "#!/bin/bash\n"), ScriptKind::Python);
    }

    // -------------------------------------------------------------------------
    // ScriptKind::stdin_invocation
    // -------------------------------------------------------------------------

    #[test]
    fn stdin_invocation_known_kinds_return_some() {
        assert_eq!(ScriptKind::Bash.stdin_invocation(), Some(("bash", ["-s"].as_slice())));
        assert_eq!(ScriptKind::Python.stdin_invocation(), Some(("python3", ["-"].as_slice())));
        assert_eq!(
            ScriptKind::NodeCommonJs.stdin_invocation(),
            Some(("node", ["--input-type=commonjs"].as_slice()))
        );
        assert_eq!(
            ScriptKind::NodeModule.stdin_invocation(),
            Some(("node", ["--input-type=module"].as_slice()))
        );
        assert_eq!(ScriptKind::TypeScript.stdin_invocation(), Some(("tsx", ["-"].as_slice())));
        assert_eq!(ScriptKind::Ruby.stdin_invocation(), Some(("ruby", ["-"].as_slice())));
    }

    #[test]
    fn stdin_invocation_unknown_is_none() {
        assert!(
            ScriptKind::Unknown
                .stdin_invocation()
                .is_none()
        );
    }

    // -------------------------------------------------------------------------
    // ScriptKind::path_invocation
    // -------------------------------------------------------------------------

    #[test]
    fn path_invocation_known_kind_prepends_interpreter() {
        let path = Path::new("/tmp/script.py");
        let (prog, args) = ScriptKind::Python.path_invocation(path);
        assert_eq!(prog, "python3");
        assert_eq!(args, vec!["/tmp/script.py"]);
    }

    #[test]
    fn path_invocation_unknown_execs_path_directly() {
        let path = Path::new("/tmp/binary");
        let (prog, args) = ScriptKind::Unknown.path_invocation(path);
        assert_eq!(prog, "/tmp/binary");
        assert!(args.is_empty());
    }
}
