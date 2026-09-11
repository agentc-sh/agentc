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
use agentc_executor_typescript::{
    error::Error,
    executor::Executor,
    guestjs::{
        errors::Error as GuestError,
        handle::{BoundClass, BoundConstructorProtocol, BoundObjectProtocol},
        marshal::{FromGuestBound, ToGuestBound},
    },
};
use async_trait::async_trait;

use crate::javascript::bindings::{
    GuestTool, Schema, Tool as GuestToolBase, ToolInput as GuestToolInput,
};

/// A JavaScript tool executed by a shared TypeScript package executor.
pub struct JavascriptTool {
    executor: Executor,
    export_name: String,
    definition: ToolDefinition,
    capabilities: CapabilitySet,
    timeout: Duration,
}

impl JavascriptTool {
    /// Creates a builder for a JavaScript tool.
    pub fn builder() -> JavascriptToolBuilder {
        JavascriptToolBuilder::new()
    }
}

#[async_trait]
impl<S> Tool<S> for JavascriptTool
where
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
        let result = tokio::time::timeout(
            self.timeout,
            self.executor.execute(move |context| {
                Box::pin(async move {
                    let (guest_input, _guard) = GuestToolInput::new(
                        input.args,
                        input
                            .state
                            .unwrap_or(serde_json::Value::Null),
                        input.emitter.map(Arc::new),
                    );

                    context
                        .guest()
                        .scope(async move |scope| {
                            context
                                .module()
                                .bind(&scope)?
                                .class_as::<GuestTool>(&export_name)?
                                .construct(())?
                                .execute(GuestToolInput::from_guest_bound(
                                    &scope,
                                    guest_input.to_guest_bound(&scope)?,
                                )?)?
                                .await
                        })
                        .await
                })
            }),
        )
        .await
        .map_err(|_| ToolError::execution_error("javascript", "tool execution timed out"))?
        .map_err(|error| {
            ToolError::sourced_execution_error("javascript", error.to_string(), Some(error))
        })?;

        let mut output = ToolOutput::ok(result.output);

        if let Some(state_update) = result.state_update {
            output = output.with_state(state_update);
        }

        Ok(output)
    }
}

/// Configures a JavaScript tool backed by a shared TypeScript package executor.
pub struct JavascriptToolBuilder {
    executor: Option<Executor>,
    export_name: Option<String>,
    capabilities: CapabilitySet,
    timeout: Duration,
}

impl JavascriptToolBuilder {
    /// Creates an empty JavaScript tool builder.
    pub fn new() -> Self {
        Self {
            executor: None,
            export_name: None,
            capabilities: CapabilitySet::default(),
            timeout: Duration::from_secs(30),
        }
    }

    /// Sets the shared package executor.
    pub fn executor(mut self, executor: Executor) -> Self {
        self.executor = Some(executor);
        self
    }

    /// Sets the package export implementing the tool.
    pub fn export_name(mut self, name: impl Into<String>) -> Self {
        self.export_name = Some(name.into());
        self
    }

    /// Adds one required tool capability.
    pub fn capability(mut self, capability: impl Into<Capability>) -> Self {
        self.capabilities
            .insert(capability.into());
        self
    }

    /// Adds required tool capabilities.
    pub fn capabilities<I, C>(mut self, capabilities: I) -> Self
    where
        I: IntoIterator<Item = C>,
        C: Into<Capability>,
    {
        self.capabilities.extend(capabilities);
        self
    }

    /// Sets the maximum duration of one tool invocation.
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = duration;
        self
    }

    /// Builds the JavaScript tool and validates its package export.
    pub async fn build(self) -> Result<JavascriptTool, Error> {
        let executor = self
            .executor
            .expect("executor must be provided to build a JavascriptTool");
        let export_name = self
            .export_name
            .expect("export_name must be provided to build a JavascriptTool");
        let definition = executor
            .execute({
                let export_name = export_name.clone();

                move |context| {
                    Box::pin(async move {
                        context
                            .guest()
                            .scope(async move |scope| {
                                let exported = context
                                    .module()
                                    .bind(&scope)?
                                    .class(&export_name)
                                    .map_err(|_| GuestError::unexpected(format!(
                                        "export '{export_name}' does not exist or is not a class",
                                    )))?;

                                if !exported
                                    .is_subclass_of(&BoundClass::of::<GuestToolBase>(&scope)?)?
                                {
                                    return Err(GuestError::unexpected(format!(
                                        "export '{export_name}' is not a Tool subclass",
                                    )));
                                }

                                Ok(ToolDefinition {
                                    name: export_name.clone(),
                                    description: exported.get::<String>("description")?,
                                    parameters: exported
                                        .get::<Schema>("parameters")?
                                        .borrow()?
                                        .document()
                                        .clone(),
                                })
                            })
                            .await
                    })
                }
            })
            .await?;

        Ok(JavascriptTool {
            executor,
            export_name,
            definition,
            capabilities: self.capabilities,
            timeout: self.timeout,
        })
    }
}

impl Default for JavascriptToolBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
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
    use agentc_executor_typescript::executor::Executor;
    use serde::{Deserialize, Serialize};
    use tokio::sync::mpsc;

    use crate::javascript::bindings::ExecutorBuilderToolsExt;
    use crate::javascript::tool::JavascriptTool;

    const TOOL_SOURCE: &str = r#"
import { Schema, Tool } from "agentc:tools";

export class Direct extends Tool {
    static readonly description = "returns a direct result";
    static readonly parameters = new Schema({ type: "object" });

    async execute(input) {
        return { output: input.args.value };
    }
}

export class Stateful extends Tool {
    static readonly description = "reads and updates state";
    static readonly parameters = new Schema({ type: "object" });

    async execute(input) {
        return {
            output: input.state.status,
            state_update: [{ op: "add", path: "/count", value: 2 }],
        };
    }
}

export class Emitter extends Tool {
    static readonly description = "emits activity";
    static readonly parameters = new Schema({ type: "object" });

    async execute(input) {
        if (!input.emit) {
            return { output: "absent" };
        }

        globalThis.retainedEmit = input.emit;
        input.emit({ activity_type: "first", patch: [] });
        input.emit({ activity_type: "second", patch: [] });

        return { output: "present" };
    }
}

export class Failure extends Tool {
    static readonly description = "throws an exception";
    static readonly parameters = new Schema({ type: "object" });

    async execute() {
        throw new Error("tool failed");
    }
}

export class Delayed extends Tool {
    static readonly description = "returns after a delay";
    static readonly parameters = new Schema({ type: "object" });

    async execute(input) {
        await new Promise((resolve) => setTimeout(resolve, input.args.delay));

        return { output: input.args.value };
    }
}

export class Synchronous extends Tool {
    static readonly description = "is not async";
    static readonly parameters = new Schema({ type: "object" });

    execute() {
        return { output: "sync" };
    }
}

export class Untyped extends Tool {
    static readonly description = "has a plain object for parameters";
    static readonly parameters = { type: "object" };

    async execute() {
        return { output: null };
    }
}

export class Unrelated {
    async execute() {
        return { output: null };
    }
}

export const plain = {
    name: "plain",
    description: "the old object contract",
    parameters: {},
    execute() {
        return { output: null };
    },
};
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
        async fn executor(workers: usize) -> Executor {
            Executor::builder("tools.ts", TOOL_SOURCE)
                .workers(workers)
                .standard_environment()
                .with_tools()
                .build()
                .await
                .unwrap()
        }

        async fn tool(executor: &Executor, export_name: &str) -> JavascriptTool {
            JavascriptTool::builder()
                .executor(executor.clone())
                .export_name(export_name)
                .build()
                .await
                .unwrap()
        }

        fn input(args: serde_json::Value) -> ToolInput<serde_json::Value> {
            ToolInput::new(
                args,
                ToolExecutionContext {
                    tenant_id: "test".to_string(),
                    session_id: Default::default(),
                    run_id: Default::default(),
                },
            )
        }

        async fn execute(
            tool: &JavascriptTool,
            input: ToolInput<serde_json::Value>,
        ) -> Result<ToolOutput<json_patch::Patch>, ToolError> {
            Tool::<TestState>::execute(tool, input).await
        }
    }

    #[tokio::test]
    async fn shared_executor_invokes_a_class_tool() {
        let executor = TestHarness::executor(1).await;
        let direct = TestHarness::tool(&executor, "Direct").await;
        let definition = Tool::<TestState>::definition(&direct);

        assert_eq!(definition.name, "Direct", "the tool's name is its export name");
        assert_eq!(definition.description, "returns a direct result");
        assert_eq!(definition.parameters, serde_json::json!({ "type": "object" }));
        assert_eq!(
            TestHarness::execute(&direct, TestHarness::input(serde_json::json!({"value": 4})))
                .await
                .unwrap()
                .output,
            serde_json::json!(4),
        );

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn builder_rejects_unknown_export() {
        let executor = TestHarness::executor(1).await;

        assert!(
            JavascriptTool::builder()
                .executor(executor.clone())
                .export_name("unknown")
                .build()
                .await
                .is_err()
        );

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn build_rejects_non_tool_export() {
        let executor = TestHarness::executor(1).await;

        for (export, message) in [
            ("Unrelated", "is not a Tool subclass"),
            ("plain", "is not a class"),
        ] {
            let error = match JavascriptTool::builder()
                .executor(executor.clone())
                .export_name(export)
                .build()
                .await
            {
                Ok(_) => panic!("a non-tool export builds"),
                Err(error) => error,
            };

            assert!(
                error.to_string().contains(message),
                "the message must name the failure and the export; got: {error}",
            );
            assert!(
                error.to_string().contains(export),
                "the message must name the export; got: {error}",
            );
        }

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn build_rejects_parameters_that_are_not_a_schema() {
        let executor = TestHarness::executor(1).await;

        assert!(
            JavascriptTool::builder()
                .executor(executor.clone())
                .export_name("Untyped")
                .build()
                .await
                .is_err()
        );

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn a_synchronous_execute_is_rejected() {
        let executor = TestHarness::executor(1).await;
        let tool = TestHarness::tool(&executor, "Synchronous").await;
        let error =
            match TestHarness::execute(&tool, TestHarness::input(serde_json::json!({}))).await {
                Ok(_) => panic!("a non-promise return succeeds"),
                Err(error) => error,
            };

        assert!(
            error
                .to_string()
                .contains("expected a promise"),
            "`Promise<ToolOutput>` is what makes async mandatory; got: {error}",
        );

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn transfers_arguments_and_state_update() {
        let executor = TestHarness::executor(1).await;
        let tool = TestHarness::tool(&executor, "Stateful").await;
        let result = TestHarness::execute(
            &tool,
            TestHarness::input(serde_json::json!({"unused": true}))
                .with_state(serde_json::json!({"status": "ready"})),
        )
        .await
        .unwrap();

        assert_eq!(result.output, serde_json::json!("ready"));
        assert_eq!(
            serde_json::to_value(result.state_update.unwrap()).unwrap(),
            serde_json::json!([{"op": "add", "path": "/count", "value": 2}]),
        );

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn activity_emitter_is_optional_and_preserves_order() {
        let executor = TestHarness::executor(1).await;
        let tool = TestHarness::tool(&executor, "Emitter").await;

        assert_eq!(
            TestHarness::execute(&tool, TestHarness::input(serde_json::json!({})),)
                .await
                .unwrap()
                .output,
            serde_json::json!("absent"),
        );

        let (sender, mut receiver) = mpsc::channel::<ActivityDelta>(2);

        assert_eq!(
            TestHarness::execute(
                &tool,
                TestHarness::input(serde_json::json!({}))
                    .with_activity_emitter(ActivityEmitter::new(sender)),
            )
            .await
            .unwrap()
            .output,
            serde_json::json!("present"),
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
        assert!(receiver.recv().await.is_none());

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn guest_exception_preserves_execution_source() {
        let executor = TestHarness::executor(1).await;
        let tool = TestHarness::tool(&executor, "Failure").await;

        assert!(matches!(
            TestHarness::execute(&tool, TestHarness::input(serde_json::json!({})),).await,
            Err(ToolError::ExecutionError { source: Some(_), .. })
        ));

        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn invocation_timeout_is_tool_specific() {
        let executor = TestHarness::executor(1).await;
        let tool = JavascriptTool::builder()
            .executor(executor.clone())
            .export_name("Delayed")
            .timeout(Duration::from_millis(1))
            .build()
            .await
            .unwrap();

        assert!(matches!(
            TestHarness::execute(
                &tool,
                TestHarness::input(serde_json::json!({
                    "delay": 20,
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

        tokio::time::sleep(Duration::from_millis(25)).await;
        executor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn concurrent_calls_share_the_package_executor() {
        let executor = TestHarness::executor(2).await;
        let tool = TestHarness::tool(&executor, "Delayed").await;
        let first = TestHarness::execute(
            &tool,
            TestHarness::input(serde_json::json!({
                "delay": 1,
                "value": "first",
            })),
        );
        let second = TestHarness::execute(
            &tool,
            TestHarness::input(serde_json::json!({
                "delay": 1,
                "value": "second",
            })),
        );
        let (first, second) = tokio::join!(first, second);

        assert_eq!(first.unwrap().output, serde_json::json!("first"));
        assert_eq!(second.unwrap().output, serde_json::json!("second"));

        executor.shutdown().await.unwrap();
    }
}
