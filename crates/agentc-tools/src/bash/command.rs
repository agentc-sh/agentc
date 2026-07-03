// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rust_bash::{CommandContext, CommandResult, VirtualCommand};

/// A [`VirtualCommand`](rust_bash::VirtualCommand) that proxies interpreter
/// dispatch to a real binary on the host system.
///
/// One instance is registered per program name listed in
/// [`CommandPolicy::Allow`](crate::bash::config::CommandPolicy::Allow). When
/// the interpreter dispatches a command with a matching name, the host binary
/// is spawned synchronously and its stdout, stderr, and exit code are returned
/// as a [`CommandResult`](rust_bash::CommandResult).
pub struct PassthroughCommand {
    name: String,
}

impl PassthroughCommand {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl VirtualCommand for PassthroughCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&self, args: &[String], _ctx: &CommandContext) -> CommandResult {
        std::process::Command::new(&self.name)
            .args(args)
            .output()
            .map(|out| CommandResult {
                stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
                exit_code: out.status.code().unwrap_or(-1),
                ..Default::default()
            })
            .unwrap_or_else(|e| CommandResult {
                stderr: format!("{}: {}", self.name, e),
                exit_code: 127,
                ..Default::default()
            })
    }
}
