// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{sync::Arc, time::Duration};

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
    backend::ExecutorBackend,
    errors::Error,
    executor::Executor,
    guestpy::handle::{Instance, ObjectProtocol},
};
use async_trait::async_trait;
use serde_json::Value;

use crate::python::bindings::{
    coercion::{Coercion, Decoded},
    guest::GuestToolClass,
    input::ToolInput as GuestToolInput,
    schema::Schema,
};

pub struct PythonTool<B: ExecutorBackend> {
    executor: Executor<B>,
    export_name: String,
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
        let export_name = self.export_name.clone();

        let Value::Object(args) = input.args else {
            return Err(ToolError::invalid_args("python tool arguments must be a JSON object"));
        };

        tokio::time::timeout(
            self.timeout,
            self.executor.execute(move |context| {
                Box::pin(async move {
                    let exported = context
                        .module()
                        .get::<GuestToolClass>(&export_name)?;
                    let coercion = Coercion::new(context.guest())?;

                    let (guest_input, _guard) = GuestToolInput::new(
                        coercion.decode(exported.args(), Value::Object(args))?,
                        input
                            .state
                            .map(|state| coercion.decode(exported.state(), state))
                            .transpose()?
                            .unwrap_or(Decoded::Json(Value::Null)),
                        input.emitter.map(Arc::new),
                    );

                    let (output, state_update) = exported
                        .class()
                        .construct(())?
                        .execute(guest_input)?
                        .await?
                        .borrow_with(|returned| {
                            (returned.output().clone(), returned.state_update().cloned())
                        })?;

                    Ok(ToolOutput {
                        output: coercion.encode(output)?,
                        state_update,
                    })
                })
            }),
        )
        .await
        .map_err(|_| ToolError::execution_error("python", "tool execution timed out"))?
        .map_err(|error| {
            ToolError::sourced_execution_error("python", error.to_string(), Some(error))
        })
    }
}

pub struct PythonToolBuilder<B: ExecutorBackend> {
    executor: Option<Executor<B>>,
    export_name: Option<String>,
    capabilities: CapabilitySet,
    timeout: Duration,
}

impl<B: ExecutorBackend> PythonToolBuilder<B> {
    pub fn new() -> Self {
        Self {
            executor: None,
            export_name: None,
            capabilities: CapabilitySet::default(),
            timeout: Duration::from_secs(30),
        }
    }

    pub fn executor(mut self, executor: Executor<B>) -> Self {
        self.executor = Some(executor);
        self
    }

    pub fn export_name(mut self, name: impl Into<String>) -> Self {
        self.export_name = Some(name.into());
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
        let export_name = self
            .export_name
            .expect("export_name must be provided to build a PythonTool");
        let definition = tokio::time::timeout(
            self.timeout,
            executor.execute({
                let export_name = export_name.clone();

                move |context| {
                    Box::pin(async move {
                        let exported = context
                            .module()
                            .get::<GuestToolClass>(&export_name)?;

                        Ok(ToolDefinition {
                            name: export_name,
                            description: exported
                                .class()
                                .get::<String>("description")?,
                            parameters: exported
                                .class()
                                .get::<Instance<_, Schema>>("parameters")?
                                .borrow_with(|schema| schema.document().clone())?,
                        })
                    })
                }
            }),
        )
        .await
        .map_err(|_| Error::unexpected("tool definition discovery timed out", None))??;

        Ok(PythonTool {
            executor,
            export_name,
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

    use crate::python::{ExecutorBuilderToolsExt, tool::PythonTool};

    const TOOL_SOURCE: &str = r#"
import asyncio
from dataclasses import dataclass

from agentc_tools import Schema, Tool, ToolInput, ToolOutput


@dataclass
class EmptyArgs:
    pass


@dataclass
class ValueArgs:
    value: int


@dataclass
class DelayArgs:
    delay: float
    value: str


@dataclass
class ReportArgs:
    city: str
    units: str = "celsius"


@dataclass
class Report:
    city: str
    units: str


@dataclass
class Status:
    status: str


class Direct(Tool[ValueArgs, int]):
    description = "returns a direct result"
    parameters = Schema({"type": "object"})

    async def execute(self, input: ToolInput[ValueArgs]) -> ToolOutput[int]:
        return ToolOutput(input.args.value)


class Double(Tool[ValueArgs, int]):
    description = "doubles the value"
    parameters = Schema({"type": "object"})

    async def execute(self, input: ToolInput[ValueArgs]) -> ToolOutput[int]:
        return ToolOutput(input.args.value * 2)


class Stateful(Tool[EmptyArgs, str]):
    description = "reads and updates state"
    parameters = Schema({"type": "object"})

    async def execute(self, input: ToolInput[EmptyArgs]) -> ToolOutput[str]:
        return ToolOutput(
            input.state["status"],
            state_update=[{"op": "add", "path": "/count", "value": 2}],
        )


class Emitter(Tool[EmptyArgs, str]):
    description = "emits activity"
    parameters = Schema({"type": "object"})

    async def execute(self, input: ToolInput[EmptyArgs]) -> ToolOutput[str]:
        if input.emit is None:
            return ToolOutput("absent")

        global retained_emit
        retained_emit = input.emit
        input.emit("first", [])
        input.emit("second", [])

        return ToolOutput("present")


class Failure(Tool[EmptyArgs, None]):
    description = "raises an exception"
    parameters = Schema({"type": "object"})

    async def execute(self, input: ToolInput[EmptyArgs]) -> ToolOutput[None]:
        raise RuntimeError("tool failed")


class Delayed(Tool[DelayArgs, str]):
    description = "returns after a delay"
    parameters = Schema({"type": "object"})

    async def execute(self, input: ToolInput[DelayArgs]) -> ToolOutput[str]:
        await asyncio.sleep(input.args.delay)

        return ToolOutput(input.args.value)


class Reporter(Tool[ReportArgs, Report]):
    description = "returns a dataclass"
    parameters = Schema({"type": "object"})

    async def execute(self, input: ToolInput[ReportArgs]) -> ToolOutput[Report]:
        return ToolOutput(Report(city=input.args.city, units=input.args.units))


class StatusReader(Tool[EmptyArgs, str, Status]):
    description = "reads a dataclass state"
    parameters = Schema({"type": "object"})

    async def execute(self, input: ToolInput[EmptyArgs, Status]) -> ToolOutput[str]:
        return ToolOutput(input.state.status)


class StateProbe(Tool[EmptyArgs, bool, Status]):
    description = "reports whether the state is absent"
    parameters = Schema({"type": "object"})

    async def execute(self, input: ToolInput[EmptyArgs, Status]) -> ToolOutput[bool]:
        return ToolOutput(input.state is None)


class CityReader(Tool[dict, str]):
    description = "reads dict arguments"
    parameters = Schema({"type": "object"})

    async def execute(self, input: ToolInput[dict]) -> ToolOutput[str]:
        return ToolOutput(input.args["city"])


class Bare(Tool):
    description = "has no type arguments"
    parameters = Schema({"type": "object"})

    async def execute(self, input: ToolInput) -> ToolOutput:
        return ToolOutput(input.args)


class Synchronous(Tool[EmptyArgs, str]):
    description = "is not async"
    parameters = Schema({"type": "object"})

    def execute(self, input: ToolInput[EmptyArgs]) -> ToolOutput[str]:
        return ToolOutput("sync")


class Untyped(Tool[EmptyArgs, None]):
    description = "has a plain dict for parameters"
    parameters = {"type": "object"}

    async def execute(self, input: ToolInput[EmptyArgs]) -> ToolOutput[None]:
        return ToolOutput(None)


class Unrelated:
    async def execute(self, input):
        return ToolOutput(None)


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
                .bundle(Bundle::single("test_tools", TOOL_SOURCE).unwrap())
                .with_tools()
                .workers(workers)
                .build()
                .await
                .unwrap()
        }

        async fn tool<B: ExecutorBackend>(
            executor: &Executor<B>,
            export_name: &str,
        ) -> PythonTool<B> {
            PythonTool::builder()
                .executor(executor.clone())
                .export_name(export_name)
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

    async fn shared_executor_dispatches_exported_tools<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;
        let direct = TestHarness::tool(&executor, "Direct").await;
        let double = TestHarness::tool(&executor, "Double").await;

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

    async fn definition_reports_the_exported_class<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;
        let tool = TestHarness::tool(&executor, "Direct").await;
        let definition = Tool::<TestState>::definition(&tool);

        assert_eq!(definition.name, "Direct");
        assert_eq!(definition.description, "returns a direct result");
        assert_eq!(definition.parameters, json!({"type": "object"}));

        executor.shutdown().await.unwrap();
    }

    async fn builder_rejects_unknown_export<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;

        assert!(
            PythonTool::<B>::builder()
                .executor(executor.clone())
                .export_name("Unknown")
                .build()
                .await
                .is_err()
        );

        executor.shutdown().await.unwrap();
    }

    async fn build_rejects_non_tool_export<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;

        for export in ["Unrelated", "Tool"] {
            let result = PythonTool::<B>::builder()
                .executor(executor.clone())
                .export_name(export)
                .build()
                .await;
            let error = match result {
                Ok(_) => panic!("a non-tool export is rejected at build"),
                Err(error) => error,
            };

            assert!(
                error
                    .to_string()
                    .contains("is not a Tool subclass"),
                "the message must name the failure; got: {error}",
            );
            assert!(
                error.to_string().contains(export),
                "the message must name the export; got: {error}",
            );
        }

        executor.shutdown().await.unwrap();
    }

    async fn build_rejects_parameters_that_are_not_a_schema<B>()
    where
        B: ExecutorBackend + Send + Sync,
    {
        let executor = TestHarness::executor::<B>(1).await;

        assert!(
            PythonTool::<B>::builder()
                .executor(executor.clone())
                .export_name("Untyped")
                .build()
                .await
                .is_err()
        );

        executor.shutdown().await.unwrap();
    }

    async fn non_object_arguments_are_rejected_before_dispatch<B>()
    where
        B: ExecutorBackend + Send + Sync,
    {
        let executor = TestHarness::executor::<B>(1).await;
        let tool = TestHarness::tool(&executor, "Direct").await;

        for args in [json!([]), json!("Paris"), Value::Null] {
            assert!(matches!(
                TestHarness::execute(&tool, TestHarness::input(args)).await,
                Err(ToolError::InvalidArguments(message))
                    if message == "python tool arguments must be a JSON object"
            ));
        }

        executor.shutdown().await.unwrap();
    }

    async fn transfers_arguments_and_state_update<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;
        let tool = TestHarness::tool(&executor, "Stateful").await;
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

    async fn a_dataclass_result_becomes_a_json_object<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;
        let tool = TestHarness::tool(&executor, "Reporter").await;
        let result = TestHarness::execute(&tool, TestHarness::input(json!({"city": "Paris"})))
            .await
            .unwrap();

        assert_eq!(result.output, json!({"city": "Paris", "units": "celsius"}));
        assert!(result.state_update.is_none());

        executor.shutdown().await.unwrap();
    }

    async fn unknown_argument_fields_are_dropped<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;
        let tool = TestHarness::tool(&executor, "Double").await;

        assert_eq!(
            TestHarness::execute(&tool, TestHarness::input(json!({"value": 2, "extra": 1})),)
                .await
                .unwrap()
                .output,
            json!(4),
        );

        executor.shutdown().await.unwrap();
    }

    async fn a_dataclass_state_is_built_from_the_state<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;
        let tool = TestHarness::tool(&executor, "StatusReader").await;

        assert_eq!(
            TestHarness::execute(
                &tool,
                TestHarness::input(json!({})).with_state(json!({"status": "ready", "count": 1})),
            )
            .await
            .unwrap()
            .output,
            json!("ready"),
        );

        executor.shutdown().await.unwrap();
    }

    async fn absent_state_is_none<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;
        let tool = TestHarness::tool(&executor, "StateProbe").await;

        assert_eq!(
            TestHarness::execute(&tool, TestHarness::input(json!({})))
                .await
                .unwrap()
                .output,
            json!(true),
        );

        executor.shutdown().await.unwrap();
    }

    async fn dict_arguments_pass_through<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;
        let tool = TestHarness::tool(&executor, "CityReader").await;

        assert_eq!(
            TestHarness::execute(&tool, TestHarness::input(json!({"city": "Paris"})))
                .await
                .unwrap()
                .output,
            json!("Paris"),
        );

        executor.shutdown().await.unwrap();
    }

    async fn a_bare_tool_receives_json<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;
        let tool = TestHarness::tool(&executor, "Bare").await;

        assert_eq!(
            TestHarness::execute(&tool, TestHarness::input(json!({"city": "Paris"})))
                .await
                .unwrap()
                .output,
            json!({"city": "Paris"}),
        );

        executor.shutdown().await.unwrap();
    }

    async fn activity_emitter_is_optional_and_preserves_order<B>()
    where
        B: ExecutorBackend + Send + Sync,
    {
        let executor = TestHarness::executor::<B>(1).await;
        let tool = TestHarness::tool(&executor, "Emitter").await;

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
        let tool = TestHarness::tool(&executor, "Emitter").await;
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
        let tool = TestHarness::tool(&executor, "Failure").await;

        assert!(matches!(
            TestHarness::execute(&tool, TestHarness::input(json!({}))).await,
            Err(ToolError::ExecutionError { source: Some(_), .. })
        ));

        executor.shutdown().await.unwrap();
    }

    async fn a_synchronous_execute_is_rejected<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;
        let tool = TestHarness::tool(&executor, "Synchronous").await;
        let result = TestHarness::execute(&tool, TestHarness::input(json!({}))).await;
        let error = match result {
            Ok(_) => panic!("a non-awaitable return is rejected"),
            Err(error) => error,
        };

        assert!(
            error
                .to_string()
                .contains("value is not awaitable"),
            "`Coroutine<B, ToolOutput<B>>` is what makes async mandatory; got: {error}",
        );

        executor.shutdown().await.unwrap();
    }

    async fn invocation_timeout_is_tool_specific<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(1).await;
        let tool = PythonTool::<B>::builder()
            .executor(executor.clone())
            .export_name("Delayed")
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

        executor.shutdown().await.unwrap();
    }

    async fn concurrent_calls_share_the_package_executor<B: ExecutorBackend + Send + Sync>() {
        let executor = TestHarness::executor::<B>(2).await;
        let tool = TestHarness::tool(&executor, "Delayed").await;
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
        shared_executor_dispatches_exported_tools,
        definition_reports_the_exported_class,
        builder_rejects_unknown_export,
        build_rejects_non_tool_export,
        build_rejects_parameters_that_are_not_a_schema,
        non_object_arguments_are_rejected_before_dispatch,
        transfers_arguments_and_state_update,
        a_dataclass_result_becomes_a_json_object,
        unknown_argument_fields_are_dropped,
        a_dataclass_state_is_built_from_the_state,
        absent_state_is_none,
        dict_arguments_pass_through,
        a_bare_tool_receives_json,
        activity_emitter_is_optional_and_preserves_order,
        retained_emit_callable_does_not_retain_the_channel,
        guest_exception_preserves_execution_source,
        a_synchronous_execute_is_rejected,
        invocation_timeout_is_tool_specific,
        concurrent_calls_share_the_package_executor,
    );
}
