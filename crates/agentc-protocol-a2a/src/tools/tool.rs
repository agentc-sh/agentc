// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use futures::TryStreamExt;
use serde_json::{
    Value,
    to_value,
};

use agentc_agent::{
    graph::state::GraphState,
    tools::{
        errors::ToolError,
        traits::TypedTool,
        types::{
            TypedToolInput,
            TypedToolOutput,
        },
    },
    types::capability::CapabilitySet,
};

use crate::{
    tools::{
        target::A2aToolTarget,
        types::{
            A2aStreamActivity,
            A2aCancelTaskToolInput,
            A2aGetTaskToolInput,
            A2aSendTaskToolInput,
        },
    },
};

#[derive(Debug, Clone)]
pub struct A2aSendTaskTool {
    target: A2aToolTarget,
    name: String,
    description: String,
}

impl A2aSendTaskTool {
    pub fn new(target: A2aToolTarget) -> Self {
        Self {
            name: target.tool_name("send"),
            description: format!(
                "Send a message to the {} over A2A and return the raw submitted task or immediate response.",
                target.name,
            ),
            target,
        }
    }
}

#[async_trait]
impl<S> TypedTool<S> for A2aSendTaskTool
where
    S: GraphState + 'static,
{
    type Input = A2aSendTaskToolInput;
    type Output = Value;
    type State = ();
    type StateUpdate = ();

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn capabilities(&self) -> CapabilitySet {
        self.target.capabilities.clone()
    }

    async fn execute(
        &self,
        input: TypedToolInput<Self::Input, Self::State>,
    ) -> Result<TypedToolOutput<Self::Output, Self::StateUpdate>, ToolError> {
        Ok(TypedToolOutput::ok(to_value(
            self.target
                .client
                .send_message(input.args.into_request(
                    &self.target,
                    &input.context,
                    Some(true),
                )?)
                .await
                .map_err(|err| self.target.operation_error("send", err.to_string()))?,
        )?))
    }
}

#[derive(Debug, Clone)]
pub struct A2aStreamTaskTool {
    target: A2aToolTarget,
    name: String,
    description: String,
}

impl A2aStreamTaskTool {
    pub fn new(target: A2aToolTarget) -> Self {
        Self {
            name: target.tool_name("stream_task"),
            description: format!(
                "Send a message to the {} over A2A, stream task progress, and return the raw stream events.",
                target.name,
            ),
            target,
        }
    }
}

#[async_trait]
impl<S> TypedTool<S> for A2aStreamTaskTool
where
    S: GraphState + 'static,
{
    type Input = A2aSendTaskToolInput;
    type Output = Value;
    type State = ();
    type StateUpdate = ();

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn capabilities(&self) -> CapabilitySet {
        self.target.capabilities.clone()
    }

    async fn execute(
        &self,
        input: TypedToolInput<Self::Input, Self::State>,
    ) -> Result<TypedToolOutput<Self::Output, Self::StateUpdate>, ToolError> {
        let mut events = Vec::new();
        let mut stream = self.target.client
            .stream_message(input.args.into_request(
                &self.target,
                &input.context,
                None,
            )?)
            .await
            .map_err(|err| self.target.operation_error("stream", err.to_string()))?;

        while let Some(response) = stream
            .try_next()
            .await
            .map_err(|err| self.target.operation_error("stream", err.to_string()))?
        {
            if let Some(emitter) = &input.emitter {
                emitter
                    .emit(A2aStreamActivity::delta(&self.target, &response)?)
                    .await;
            }

            let is_terminal = A2aStreamActivity::is_terminal(&response);

            events.push(to_value(response)?);

            if is_terminal {
                break;
            }
        }

        Ok(TypedToolOutput::ok(Value::Array(events)))
    }
}

#[derive(Debug, Clone)]
pub struct A2aGetTaskTool {
    target: A2aToolTarget,
    name: String,
    description: String,
}

impl A2aGetTaskTool {
    pub fn new(target: A2aToolTarget) -> Self {
        Self {
            name: target.tool_name("get_task"),
            description: format!(
                "Retrieve the raw current state and artifacts for a {} A2A task by task ID.",
                target.name,
            ),
            target,
        }
    }
}

#[async_trait]
impl<S> TypedTool<S> for A2aGetTaskTool
where
    S: GraphState + 'static,
{
    type Input = A2aGetTaskToolInput;
    type Output = Value;
    type State = ();
    type StateUpdate = ();

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn capabilities(&self) -> CapabilitySet {
        self.target.capabilities.clone()
    }

    async fn execute(
        &self,
        input: TypedToolInput<Self::Input, Self::State>,
    ) -> Result<TypedToolOutput<Self::Output, Self::StateUpdate>, ToolError> {
        Ok(TypedToolOutput::ok(to_value(
            self.target
                .client
                .get_task(input.args.into_request(&self.target, &input.context))
                .await
                .map_err(|err| self.target.operation_error("get_task", err.to_string()))?,
        )?))
    }
}

#[derive(Debug, Clone)]
pub struct A2aCancelTaskTool {
    target: A2aToolTarget,
    name: String,
    description: String,
}

impl A2aCancelTaskTool {
    pub fn new(target: A2aToolTarget) -> Self {
        Self {
            name: target.tool_name("cancel_task"),
            description: format!(
                "Cancel a {} A2A task by task ID and return the raw task response.",
                target.name,
            ),
            target,
        }
    }
}

#[async_trait]
impl<S> TypedTool<S> for A2aCancelTaskTool
where
    S: GraphState + 'static,
{
    type Input = A2aCancelTaskToolInput;
    type Output = Value;
    type State = ();
    type StateUpdate = ();

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn capabilities(&self) -> CapabilitySet {
        self.target.capabilities.clone()
    }

    async fn execute(
        &self,
        input: TypedToolInput<Self::Input, Self::State>,
    ) -> Result<TypedToolOutput<Self::Output, Self::StateUpdate>, ToolError> {
        Ok(TypedToolOutput::ok(to_value(
            self.target
                .client
                .cancel_task(input.args.into_request(&self.target, &input.context)?)
                .await
                .map_err(|err| {
                    self.target
                        .operation_error("cancel_task", err.to_string())
                })?,
        )?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use agentc_agent::{
        graph::state::{
            GraphState,
            GraphStateInput,
            GraphStateUpdate,
        },
        tools::{
            activity::ActivityEmitter,
            traits::TypedTool,
            types::{
                ToolExecutionContext,
                TypedToolInput,
            },
        },
    };
    use serde::{
        Deserialize,
        Serialize,
    };
    use tokio::sync::mpsc;
    use uuid::Uuid;
    use wiremock::{
        Mock,
        MockServer,
        ResponseTemplate,
        matchers::{
            method,
            path,
        },
    };

    use crate::{
        client::{
            A2aClient,
            A2aClientConfig,
        },
        protocol::{
            StreamResponse,
            Task,
            TaskId,
            TaskState,
            TaskStatus,
        },
        tools::{
            target::A2aToolTarget,
            types::{
                A2aSendTaskToolInput,
                A2aSendTaskToolInputMessage,
            },
        },
    };

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    struct DummyState;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    struct DummyStateUpdate;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    struct DummyStateInput;

    impl GraphState for DummyState {
        type Update = DummyStateUpdate;
        type Input = DummyStateInput;
    }

    impl GraphStateUpdate for DummyStateUpdate {
        type State = DummyState;

        fn apply(self, _update: &mut Self::State) {}

        fn merge(self, _other: Self) -> Self {
            self
        }
    }

    impl GraphStateInput for DummyStateInput {
        type State = DummyState;

        fn initialize(self) -> Self::State {
            DummyState
        }
    }

    struct ToolFixture;

    impl ToolFixture {
        async fn target() -> (MockServer, A2aToolTarget) {
            let server = MockServer::start().await;
            let client = A2aClient::new(A2aClientConfig::new(server.uri()))
                .expect("client config should be valid");

            (
                server,
                A2aToolTarget::builder()
                    .id("planner")
                    .client(client)
                    .build()
                    .expect("target should build"),
            )
        }

        fn input() -> A2aSendTaskToolInput {
            A2aSendTaskToolInput {
                message: A2aSendTaskToolInputMessage {
                    text: Some("plan this".to_string()),
                    data: None,
                },
                context_id: None,
                task_id: None,
                metadata: None,
                accepted_output_modes: None,
                history_length: None,
            }
        }

        fn context() -> ToolExecutionContext {
            ToolExecutionContext {
                tenant_id: "tenant-1".to_string(),
                session_id: Uuid::nil(),
                run_id: Uuid::nil(),
            }
        }

        fn completed_task() -> Task {
            Task {
                id: TaskId::new("task-1"),
                context_id: "context-1".to_string(),
                status: TaskStatus {
                    state: TaskState::Completed,
                    message: None,
                    timestamp: None,
                },
                artifacts: None,
                history: None,
                metadata: None,
            }
        }
    }

    #[tokio::test]
    async fn stream_task_tool_returns_events_and_emits_activity() {
        let (server, target) = ToolFixture::target().await;

        Mock::given(method("POST"))
            .and(path("/message:stream"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(format!(
                    "data: {}\n\n",
                    serde_json::to_string(&StreamResponse::Task(
                        ToolFixture::completed_task(),
                    ))
                    .expect("stream response should serialize"),
                )),
            )
            .mount(&server)
            .await;

        let (tx, mut rx) = mpsc::channel(4);
        let tool = target.stream_task_tool();

        let output = <A2aStreamTaskTool as TypedTool<DummyState>>::execute(
            &tool,
            TypedToolInput::new(ToolFixture::input(), ToolFixture::context())
                .with_activity_emitter(ActivityEmitter::new(tx)),
        )
        .await
        .expect("tool should execute");

        assert_eq!(
            output
                .output
                .as_array()
                .map(Vec::len),
            Some(1)
        );
        assert_eq!(
            rx.recv()
                .await
                .expect("activity should be emitted")
                .activity_type,
            "a2a_task"
        );
    }
}
