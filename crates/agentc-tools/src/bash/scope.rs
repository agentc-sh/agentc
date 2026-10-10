// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{path::PathBuf as BashkitPathBuf, sync::Arc};

use agentc_fs::fs::Fs;
use agentc_http::client::HttpClient;
use async_trait::async_trait;
use bashkit::{Bash, ExecResult};
use tokio::sync::Mutex;

use crate::bash::{
    config::{BashConfig, CommandPolicy},
    curl::Curl,
    errors::BashToolError,
    fs::BashkitFs,
    passthrough::PassthroughCommand,
};

pub struct BashFactory {
    fs: Fs,
    http: HttpClient,
    config: BashConfig,
}

impl BashFactory {
    pub fn new(fs: Fs, http: HttpClient, config: BashConfig) -> Self {
        BashFactory { fs, http, config }
    }

    pub fn build(&self) -> Bash {
        let mut builder = Bash::builder()
            .fs(Arc::new(BashkitFs::new(self.fs.clone())))
            .cwd(BashkitPathBuf::from(&self.config.cwd))
            .limits((&self.config.limits).into())
            .builtin("curl", Box::new(Curl::new(self.http.clone())));

        for (key, value) in self.config.env_policy.resolve() {
            builder = builder.env(key, value);
        }

        if let CommandPolicy::Allow(commands) = &self.config.command_policy {
            for command in commands {
                builder = builder.builtin(command, Box::new(PassthroughCommand::new(command)));
            }
        }

        builder.build()
    }
}

#[async_trait]
pub trait ShellScope: Send + Sync {
    async fn run(&self, script: &str) -> Result<ExecResult, BashToolError>;
}

pub struct FactoryScope {
    factory: BashFactory,
}

impl FactoryScope {
    pub fn new(factory: BashFactory) -> Self {
        FactoryScope { factory }
    }
}

#[async_trait]
impl ShellScope for FactoryScope {
    async fn run(&self, script: &str) -> Result<ExecResult, BashToolError> {
        let mut bash = self.factory.build();

        bash.exec(script)
            .await
            .map_err(BashToolError::execution)
    }
}

pub struct SharedScope {
    bash: Mutex<Bash>,
}

impl SharedScope {
    pub fn new(factory: BashFactory) -> Self {
        SharedScope { bash: Mutex::new(factory.build()) }
    }
}

#[async_trait]
impl ShellScope for SharedScope {
    async fn run(&self, script: &str) -> Result<ExecResult, BashToolError> {
        self.bash
            .lock()
            .await
            .exec(script)
            .await
            .map_err(BashToolError::execution)
    }
}

#[cfg(test)]
mod tests {
    use agentc_fs::fs::Fs;
    use agentc_http::client::HttpClient;

    use crate::bash::{
        config::BashConfig,
        scope::{BashFactory, FactoryScope, SharedScope, ShellScope},
    };

    fn factory(fs: Fs) -> BashFactory {
        BashFactory::new(fs, HttpClient::builder().build().unwrap(), BashConfig::default())
    }

    #[tokio::test]
    async fn factory_scope_does_not_retain_shell_variables() {
        let scope = FactoryScope::new(factory(Fs::memory()));

        scope.run("value=first").await.unwrap();

        assert_eq!(
            scope
                .run("printf %s \"$value\"")
                .await
                .unwrap()
                .stdout
                .as_bytes(),
            b""
        );
    }

    #[tokio::test]
    async fn shared_scope_retains_shell_variables() {
        let scope = SharedScope::new(factory(Fs::memory()));

        scope.run("value=first").await.unwrap();

        assert_eq!(
            scope
                .run("printf %s \"$value\"")
                .await
                .unwrap()
                .stdout
                .as_bytes(),
            b"first"
        );
    }

    #[tokio::test]
    async fn scopes_share_filesystem_state() {
        let fs = Fs::memory();
        let scope = FactoryScope::new(factory(fs.clone()));

        scope
            .run("printf content > /shared.txt")
            .await
            .unwrap();

        assert_eq!(
            fs.root()
                .open_file("/shared.txt")
                .await
                .unwrap()
                .read_to_end()
                .await
                .unwrap(),
            b"content"
        );
    }

    #[tokio::test]
    async fn factory_applies_the_configured_working_directory() {
        let fs = Fs::memory();

        fs.root()
            .create_dir_all("/workspace")
            .await
            .unwrap();

        let config = BashConfig {
            cwd: String::from("/workspace"),
            ..Default::default()
        };

        assert_eq!(
            FactoryScope::new(
                BashFactory::new(fs, HttpClient::builder().build().unwrap(), config,)
            )
            .run("pwd")
            .await
            .unwrap()
            .stdout
            .text_lossy(),
            "/workspace\n"
        );
    }
}
