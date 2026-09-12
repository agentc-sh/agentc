// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_agent::{
    graph::state::GraphState,
    tools::{
        errors::ToolError,
        traits::TypedTool,
        types::{TypedToolInput, TypedToolOutput},
    },
    types::capability::CapabilitySet,
};
use agentc_fs::fs::Fs;
use agentc_http::client::HttpClient;
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::bash::{
    config::{BashConfig, CommandPolicy, EnvPolicy, ExecLimits},
    scope::{BashFactory, FactoryScope, SharedScope, ShellScope},
};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BashInput {
    pub command: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct BashOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub struct BashTool {
    scope: Box<dyn ShellScope>,
}

impl BashTool {
    pub fn builder(fs: Fs, http: HttpClient) -> BashToolBuilder {
        BashToolBuilder::new(fs, http)
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
        "Invoke bash commands and scripts."
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet::from(["bash"])
    }

    async fn execute(
        &self,
        input: TypedToolInput<BashInput>,
    ) -> Result<TypedToolOutput<BashOutput, ()>, ToolError> {
        let result = self
            .scope
            .run(&input.args.command)
            .await?;

        Ok(TypedToolOutput::ok(BashOutput {
            stdout: result.stdout.text_lossy().into_owned(),
            stderr: result.stderr.text_lossy().into_owned(),
            exit_code: result.exit_code,
        }))
    }
}

pub struct BashToolBuilder {
    fs: Fs,
    http: HttpClient,
    config: BashConfig,
    shared: bool,
}

impl BashToolBuilder {
    pub fn new(fs: Fs, http: HttpClient) -> Self {
        BashToolBuilder {
            fs,
            http,
            config: BashConfig::default(),
            shared: false,
        }
    }

    pub fn command_policy(mut self, policy: CommandPolicy) -> Self {
        self.config.command_policy = policy;
        self
    }

    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.config.cwd = cwd.into();
        self
    }

    pub fn env_policy(mut self, policy: EnvPolicy) -> Self {
        self.config.env_policy = policy;
        self
    }

    pub fn limits(mut self, limits: ExecLimits) -> Self {
        self.config.limits = limits;
        self
    }

    pub fn shared(mut self) -> Self {
        self.shared = true;
        self
    }

    pub fn build(self) -> BashTool {
        let factory = BashFactory::new(self.fs, self.http, self.config);

        BashTool {
            scope: if self.shared {
                Box::new(SharedScope::new(factory))
            } else {
                Box::new(FactoryScope::new(factory))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use agentc_agent::{
        graph::state::{GraphState, GraphStateInput, GraphStateUpdate},
        tools::{
            traits::TypedTool,
            types::{ToolExecutionContext, TypedToolInput},
        },
    };
    use agentc_fs::fs::Fs;
    use agentc_http::client::{
        HttpClient,
        policies::{PatternPolicy, UrlPattern},
    };
    use serde::{Deserialize, Serialize};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    use uuid::Uuid;

    use crate::bash::tool::{BashInput, BashTool};

    #[derive(Clone, Debug, Deserialize, Serialize)]
    struct TestState;

    impl GraphState for TestState {
        type Update = TestStateUpdate;
        type Input = TestStateInput;
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    struct TestStateUpdate;

    impl GraphStateUpdate for TestStateUpdate {
        type State = TestState;

        fn apply(self, _state: &mut Self::State) {}

        fn merge(self, _other: Self) -> Self {
            self
        }
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct TestStateInput;

    impl GraphStateInput for TestStateInput {
        type State = TestState;

        fn initialize(self) -> Self::State {
            TestState
        }
    }

    fn context() -> ToolExecutionContext {
        ToolExecutionContext {
            tenant_id: String::from("tenant"),
            session_id: Uuid::new_v4(),
            run_id: Uuid::new_v4(),
        }
    }

    #[tokio::test]
    async fn preserves_the_tool_contract_and_converts_output() {
        let tool = BashTool::builder(Fs::memory(), HttpClient::builder().build().unwrap()).build();

        assert_eq!(TypedTool::<TestState>::name(&tool), "bash");
        assert_eq!(TypedTool::<TestState>::description(&tool), "Invoke bash commands and scripts.",);
        assert_eq!(TypedTool::<TestState>::capabilities(&tool), ["bash"].into());

        let output = TypedTool::<TestState>::execute(
            &tool,
            TypedToolInput::new(
                BashInput {
                    command: String::from("printf output; printf error >&2; exit 7"),
                },
                context(),
            ),
        )
        .await
        .unwrap()
        .output;

        assert_eq!(output.stdout, "output");
        assert_eq!(output.stderr, "error");
        assert_eq!(output.exit_code, 7);
    }

    #[tokio::test]
    async fn converges_filesystem_and_curl_through_the_tool_contract() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap();
        let address = listener
            .local_addr()
            .unwrap()
            .to_string();

        let handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buffer = [0u8; 1024];

            let bytes_read = stream.read(&mut buffer).await.unwrap();
            assert!(bytes_read > 0);

            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\nconverged",
                )
                .await
                .unwrap();
            stream.shutdown().await.unwrap();
        });

        let fs = Fs::memory();
        let client = HttpClient::builder()
            .policy(
                PatternPolicy::allow([UrlPattern::parse(format!("http://{address}/*")).unwrap()])
                    .unwrap(),
            )
            .build()
            .unwrap();
        let tool = BashTool::builder(fs.clone(), client).build();

        let output = TypedTool::<TestState>::execute(
            &tool,
            TypedToolInput::new(
                BashInput {
                    command: format!(
                        "curl -o /download.txt http://{address}/converged && cat /download.txt"
                    ),
                },
                context(),
            ),
        )
        .await
        .unwrap()
        .output;

        assert_eq!(output.stdout, "converged");
        assert_eq!(
            fs.root()
                .open_file("/download.txt")
                .await
                .unwrap()
                .read_to_end()
                .await
                .unwrap(),
            b"converged"
        );

        handle.await.unwrap();
    }
}
