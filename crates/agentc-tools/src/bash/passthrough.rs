// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use bashkit::{Builtin, BuiltinContext, ExecResult};
use tokio::process::Command;

pub struct PassthroughCommand {
    name: String,
}

impl PassthroughCommand {
    pub fn new(name: impl Into<String>) -> Self {
        PassthroughCommand { name: name.into() }
    }
}

#[async_trait]
impl Builtin for PassthroughCommand {
    async fn execute(&self, context: BuiltinContext<'_>) -> bashkit::Result<ExecResult> {
        Ok(
            match Command::new(&self.name)
                .args(context.args)
                .output()
                .await
            {
                Ok(output) => ExecResult {
                    stdout: output.stdout.into(),
                    stderr: output.stderr.into(),
                    exit_code: output.status.code().unwrap_or(-1),
                    ..Default::default()
                },
                Err(error) => ExecResult::err(format!("{}: {error}\n", self.name), 127),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use bashkit::Bash;

    use crate::bash::passthrough::PassthroughCommand;

    #[tokio::test]
    async fn executes_an_allowed_host_command() {
        let mut bash = Bash::builder()
            .builtin("rustc", Box::new(PassthroughCommand::new("rustc")))
            .build();

        let result = bash
            .exec("rustc --version")
            .await
            .unwrap();

        assert_eq!(result.exit_code, 0);
        assert!(
            result
                .stdout
                .text_lossy()
                .starts_with("rustc ")
        );
        assert!(result.stderr.as_bytes().is_empty());
    }

    #[tokio::test]
    async fn reports_a_missing_host_command_as_exit_127() {
        let name = "agentc-command-that-does-not-exist";
        let mut bash = Bash::builder()
            .builtin(name, Box::new(PassthroughCommand::new(name)))
            .build();

        let result = bash.exec(name).await.unwrap();

        assert_eq!(result.exit_code, 127);
        assert!(
            result
                .stderr
                .text_lossy()
                .contains(name)
        );
    }
}
