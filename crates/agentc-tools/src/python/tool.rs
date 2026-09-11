// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::time::Duration;

use agentc_agent::{
    graph::state::GraphState,
    tools::{
        errors::ToolError,
        traits::Tool,
        types::{ToolInput, ToolOutput},
    },
    types::{
        capability::{Capability, CapabilitySet},
        tools::ToolDefinition,
    },
};
use agentc_executor_python::{
    backend::ExecutorBackend, errors::Error, executor::Executor, guestpy::handle::ObjectProtocol,
};
use async_trait::async_trait;

use crate::python::types::{PythonToolDefinition, PythonToolInput, PythonToolResult};

pub struct PythonTool<B: ExecutorBackend> {
    executor: Executor<B>,
    tool_name: String,
    definition: ToolDefinition,
    capabilities: CapabilitySet,
    timeout: Duration,
}

impl<B: ExecutorBackend> PythonTool<B> {
    pub fn builder() -> PythonToolBuilder<B> {
        PythonToolBuilder::new()
    }
}

#[async_trait]
impl<B, S> Tool<S> for PythonTool<B>
where
    B: ExecutorBackend + Send + Sync,
    S: GraphState + 'static,
    S::Update: Default,
{
    type State = serde_json::Value;
    type StateUpdate = json_patch::Patch;

    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    fn capabilities(&self) -> CapabilitySet {
        self.capabilities.clone()
    }

    async fn execute(
        &self,
        input: ToolInput<Self::State>,
    ) -> Result<ToolOutput<Self::StateUpdate>, ToolError> {
        let tool_name = self.tool_name.clone();
        let result = tokio::time::timeout(
            self.timeout,
            self.executor.execute(move |context| {
                Box::pin(async move {
                    let (positional, keyword, _emitter) =
                        PythonToolInput::new(&tool_name, input.args, input.state, input.emitter)
                            .into_parts();

                    context
                        .guest()
                        .import("agentc_tdk")?
                        .function("invoke_tool")?
                        .call_with::<_, _, PythonToolResult>(positional, keyword)
                })
            }),
        )
        .await
        .map_err(|_| ToolError::execution_error("python", "tool execution timed out"))?
        .map_err(|error| {
            ToolError::sourced_execution_error("python", error.to_string(), Some(error))
        })?;

        let mut output = ToolOutput::ok(result.output);

        if let Some(state_update) = result.state_update {
            output = output.with_state(state_update);
        }

        Ok(output)
    }
}

pub struct PythonToolBuilder<B: ExecutorBackend> {
    executor: Option<Executor<B>>,
    tool_name: Option<String>,
    capabilities: CapabilitySet,
    timeout: Duration,
}

impl<B: ExecutorBackend> PythonToolBuilder<B> {
    pub fn new() -> Self {
        Self {
            executor: None,
            tool_name: None,
            capabilities: CapabilitySet::default(),
            timeout: Duration::from_secs(30),
        }
    }

    pub fn executor(mut self, executor: Executor<B>) -> Self {
        self.executor = Some(executor);
        self
    }

    pub fn tool_name(mut self, name: impl Into<String>) -> Self {
        self.tool_name = Some(name.into());
        self
    }

    pub fn capability(mut self, capability: impl Into<Capability>) -> Self {
        self.capabilities
            .insert(capability.into());
        self
    }

    pub fn capabilities<I, C>(mut self, capabilities: I) -> Self
    where
        I: IntoIterator<Item = C>,
        C: Into<Capability>,
    {
        self.capabilities.extend(capabilities);
        self
    }

    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = duration;
        self
    }

    pub async fn build(self) -> Result<PythonTool<B>, Error> {
        let executor = self
            .executor
            .expect("executor must be provided to build a PythonTool");
        let tool_name = self
            .tool_name
            .expect("tool_name must be provided to build a PythonTool");
        let definition = tokio::time::timeout(
            self.timeout,
            executor.execute({
                let tool_name = tool_name.clone();

                move |context| {
                    Box::pin(async move {
                        context
                            .guest()
                            .import("agentc_tdk")?
                            .function("get_tool_definition")?
                            .call::<_, PythonToolDefinition>((tool_name,))
                    })
                }
            }),
        )
        .await
        .map_err(|_| Error::unexpected("tool definition discovery timed out", None))??
        .into_definition(&tool_name);

        Ok(PythonTool {
            executor,
            tool_name,
            definition,
            capabilities: self.capabilities,
            timeout: self.timeout,
        })
    }
}

impl<B: ExecutorBackend> Default for PythonToolBuilder<B> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, any(feature = "python-rustpython", feature = "python-cpython")))]
mod tests {
    use std::time::Duration;

    use agentc_agent::{
        graph::state::{GraphState, GraphStateInput, GraphStateUpdate},
        tools::{
            activity::{ActivityDelta, ActivityEmitter},
            errors::ToolError,
            traits::Tool,
            types::{ToolExecutionContext, ToolInput, ToolOutput},
        },
    };
    use agentc_executor_python::{
        backend::ExecutorBackend,
        executor::Executor,
        guestpy::{bundle::Bundle, handle::ObjectProtocol},
    };
    use serde::{Deserialize, Serialize};
    use serde_json::{Value, json};
    use tokio::sync::mpsc;

    use crate::python::tool::PythonTool;

    const AGENTC_TDK_STUB: &str = r#"
from dataclasses import asdict, dataclass
from typing import Any, ClassVar, Optional

__tool_registry__ = {}

@dataclass
class Args: ...

@dataclass
class ToolInput:
    args: Any
    state: Any = None
    emit: Any = None

@dataclass
class ToolOutput:
    output: Any
    state_update: Any = None

    def to_dict(self):
        return asdict(self)

class Tool:
    args: type
    state: type | None = None
    name: ClassVar[str]
    description: ClassVar[str]
    schema: ClassVar[dict]

    def __init_subclass__(cls, **kwargs):
        super().__init_subclass__(**kwargs)
        if hasattr(cls, "name"):
            __tool_registry__[cls.name] = cls

    def invoke(self, args, state=None, emit=None):
        typed_state = None
        if state is not None and self.state is not None and isinstance(state, dict):
            typed_state = self.state(**state)
        elif state is not None and self.state is None:
            typed_state = state
        return self.execute(ToolInput(self.args(**args), state=typed_state, emit=emit)).to_dict()

    def execute(self, input):
        raise NotImplementedError()

def get_tool_definition(name):
    cls = __tool_registry__[name]
    return {"name": cls.name, "description": cls.description, "schema": cls.schema}

def invoke_tool(name, args, state=None, emit=None):
    return __tool_registry__[name]().invoke(args, state=state, emit=emit)
"#;

    const TOOL_SOURCE: &str = r#"
from agentc_tdk import Tool, Args, ToolOutput
from dataclasses import dataclass
import time


@dataclass
class EmptyArgs(Args): ...


@dataclass
class ValueArgs(Args):
    value: int


@dataclass
class DelayArgs(Args):
    delay: float
    value: str


class DirectTool(Tool):
    args = ValueArgs
    name = "direct"
    description = "returns a direct result"
    schema = {}

    def execute(self, input):
        return ToolOutput(output=input.args.value)


class DoubleTool(Tool):
    args = ValueArgs
    name = "double"
    description = "doubles the value"
    schema = {}

    def execute(self, input):
        return ToolOutput(output=input.args.value * 2)


class StateTool(Tool):
    args = EmptyArgs
    name = "state"
    description = "reads and updates state"
    schema = {}

    def execute(self, input):
        return ToolOutput(
            output=input.state["status"],
            state_update=[{"op": "add", "path": "/count", "value": 2}],
        )


class EmitterTool(Tool):
    args = EmptyArgs
    name = "emitter"
    description = "emits activity"
    schema = {}

    def execute(self, input):
        if input.emit is None:
            return ToolOutput(output="absent")

        global retained_emit
        retained_emit = input.emit
        input.emit("first", [])
        input.emit("second", [])
        return ToolOutput(output="present")


class FailureTool(Tool):
    args = EmptyArgs
    name = "failure"
    description = "raises an exception"
    schema = {}

    def execute(self, input):
        raise RuntimeError("tool failed")


class DelayedTool(Tool):
    args = DelayArgs
    name = "delayed"
    description = "returns after a delay"
    schema = {}

    def execute(self, input):
        time.sleep(input.args.delay)
        return ToolOutput(output=input.args.value)


def call_retained():
    retained_emit("third", [])
"#;

    #[derive(Clone, Debug, Default, Deserialize, Serialize)]
    struct TestState;

    #[derive(Clone, Debug, Default, Deserialize, Serialize)]
    struct TestStateUpdate;

    #[derive(Clone, Debug, Deserialize, Serialize)]
    struct TestStateInput;

    impl GraphState for TestState {
        type Update = TestStateUpdate;
        type Input = TestStateInput;
    }

    impl GraphStateUpdate for TestStateUpdate {
        type State = TestState;

        fn apply(self, _state: &mut Self::State) {}

        fn merge(self, _other: Self) -> Self {
            self
        }
    }

    impl GraphStateInput for TestStateInput {
        type State = TestState;

        fn initialize(self) -> Self::State {
            TestState
        }
    }

    struct TestHarness;

    impl TestHarness {
        async fn executor<B: ExecutorBackend>(workers: usize) -> Executor<B> {
            Executor::<B>::builder("test_tools")
                .bundle(Bundle::single("agentc_tdk", AGENTC_TDK_STUB).unwrap())
                .bundle(Bundle::single("test_tools", TOOL_SOURCE).unwrap())
                .workers(workers)
                .build()
                .await
                .unwrap()
        }

        async fn tool<B: ExecutorBackend>(
            executor: &Executor<B>,
            tool_name: &str,
        ) -> PythonTool<B> {
            PythonTool::builder()
                .executor(executor.clone())
                .tool_name(tool_name)
                .build()
                .await
                .unwrap()
        }

        fn input(args: Value) -> ToolInput<Value> {
            ToolInput::new(
                args,
                ToolExecutionContext {
                    tenant_id: "test".to_string(),
                    session_id: Default::default(),
                    run_id: Default::default(),
                },
            )
        }

        async fn execute<B: ExecutorBackend + Send + Sync>(
            tool: &PythonTool<B>,
            input: ToolInput<Value>,
        ) -> Result<ToolOutput<json_patch::Patch>, ToolError> {
            Tool::<TestState>::execute(tool, input).await
        }
    }

    macro_rules! parameterized {
        ($($name:ident),+ $(,)?) => {
            #[cfg(feature = "python-rustpython")]
            mod rustpython {
                use agentc_executor_python::guestpy::rustpython::RustPython;

                $(
                    #[tokio::test]
                    async fn $name() {
                        crate::python::tool::tests::$name::<RustPython>().await;
                    }
                )+
            }

            #[cfg(feature = "python-cpython")]
            mod cpython {
                use agentc_executor_python::guestpy::pyo3::CPython;

                $(
                    #[tokio::test]
                    async fn $name() {
                        crate::python::tool::tests::$name::<CPython>().await;
                    }
                )+
            }
        };
    }

    async fn shared_executor_dispatches_registered_tools<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;
        let direct = TestHarness::tool(&executor, "direct").await;
        let double = TestHarness::tool(&executor, "double").await;

        assert_eq!(
            TestHarness::execute(&direct, TestHarness::input(json!({"value": 4})))
                .await
                .unwrap()
                .output,
            json!(4),
        );
        assert_eq!(
            TestHarness::execute(&double, TestHarness::input(json!({"value": 4})))
                .await
                .unwrap()
                .output,
            json!(8),
        );

        executor.shutdown().await.unwrap();
    }

    async fn definition_reports_registered_metadata<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;
        let tool = TestHarness::tool(&executor, "direct").await;
        let definition = Tool::<TestState>::definition(&tool);

        assert_eq!(definition.name, "direct");
        assert_eq!(definition.description, "returns a direct result");
        assert_eq!(definition.parameters, json!({}));

        executor.shutdown().await.unwrap();
    }

    async fn builder_rejects_unregistered_tool_name<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;

        assert!(
            PythonTool::<B>::builder()
                .executor(executor.clone())
                .tool_name("unknown")
                .build()
                .await
                .is_err()
        );

        executor.shutdown().await.unwrap();
    }

    async fn transfers_arguments_and_state_update<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;
        let tool = TestHarness::tool(&executor, "state").await;
        let result = TestHarness::execute(
            &tool,
            TestHarness::input(json!({})).with_state(json!({"status": "ready"})),
        )
        .await
        .unwrap();

        assert_eq!(result.output, json!("ready"));
        assert_eq!(
            serde_json::to_value(result.state_update.unwrap()).unwrap(),
            json!([{"op": "add", "path": "/count", "value": 2}]),
        );

        executor.shutdown().await.unwrap();
    }

    async fn activity_emitter_is_optional_and_preserves_order<B>()
    where
        B: ExecutorBackend + Send + Sync,
    {
        let executor = TestHarness::executor::<B>(1).await;
        let tool = TestHarness::tool(&executor, "emitter").await;

        assert_eq!(
            TestHarness::execute(&tool, TestHarness::input(json!({})))
                .await
                .unwrap()
                .output,
            json!("absent"),
        );

        let (sender, mut receiver) = mpsc::channel::<ActivityDelta>(2);

        assert_eq!(
            TestHarness::execute(
                &tool,
                TestHarness::input(json!({})).with_activity_emitter(ActivityEmitter::new(sender)),
            )
            .await
            .unwrap()
            .output,
            json!("present"),
        );
        assert_eq!(
            receiver
                .recv()
                .await
                .unwrap()
                .activity_type,
            "first",
        );
        assert_eq!(
            receiver
                .recv()
                .await
                .unwrap()
                .activity_type,
            "second",
        );

        executor.shutdown().await.unwrap();
    }

    async fn retained_emit_callable_does_not_retain_the_channel<B>()
    where
        B: ExecutorBackend + Send + Sync,
    {
        let executor = TestHarness::executor::<B>(1).await;
        let tool = TestHarness::tool(&executor, "emitter").await;
        let (sender, mut receiver) = mpsc::channel::<ActivityDelta>(2);

        TestHarness::execute(
            &tool,
            TestHarness::input(json!({})).with_activity_emitter(ActivityEmitter::new(sender)),
        )
        .await
        .unwrap();

        receiver.recv().await.unwrap();
        receiver.recv().await.unwrap();

        executor
            .execute(|context| {
                Box::pin(async move {
                    context
                        .guest()
                        .import("test_tools")?
                        .function("call_retained")?
                        .call::<_, ()>(())
                })
            })
            .await
            .unwrap();

        assert!(receiver.recv().await.is_none());

        executor.shutdown().await.unwrap();
    }

    async fn guest_exception_preserves_execution_source<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;
        let tool = TestHarness::tool(&executor, "failure").await;

        assert!(matches!(
            TestHarness::execute(&tool, TestHarness::input(json!({}))).await,
            Err(ToolError::ExecutionError { source: Some(_), .. })
        ));

        executor.shutdown().await.unwrap();
    }

    async fn invocation_timeout_is_tool_specific<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;
        let tool = PythonTool::<B>::builder()
            .executor(executor.clone())
            .tool_name("delayed")
            .timeout(Duration::from_millis(100))
            .build()
            .await
            .unwrap();

        assert!(matches!(
            TestHarness::execute(
                &tool,
                TestHarness::input(json!({
                    "delay": 0.2,
                    "value": "late",
                })),
            )
            .await,
            Err(ToolError::ExecutionError {
                message,
                source: None,
                ..
            }) if message == "tool execution timed out"
        ));

        tokio::time::sleep(Duration::from_millis(250)).await;
        executor.shutdown().await.unwrap();
    }

    async fn concurrent_calls_share_the_package_executor<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(2).await;
        let tool = TestHarness::tool(&executor, "delayed").await;
        let (first, second) = tokio::join!(
            TestHarness::execute(
                &tool,
                TestHarness::input(json!({
                    "delay": 0.05,
                    "value": "first",
                })),
            ),
            TestHarness::execute(
                &tool,
                TestHarness::input(json!({
                    "delay": 0.05,
                    "value": "second",
                })),
            ),
        );

        assert_eq!(first.unwrap().output, json!("first"));
        assert_eq!(second.unwrap().output, json!("second"));

        executor.shutdown().await.unwrap();
    }

    parameterized!(
        shared_executor_dispatches_registered_tools,
        definition_reports_registered_metadata,
        builder_rejects_unregistered_tool_name,
        transfers_arguments_and_state_update,
        activity_emitter_is_optional_and_preserves_order,
        retained_emit_callable_does_not_retain_the_channel,
        guest_exception_preserves_execution_source,
        invocation_timeout_is_tool_specific,
        concurrent_calls_share_the_package_executor,
    );
}
