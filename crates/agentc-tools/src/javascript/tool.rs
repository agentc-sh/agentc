// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use json_patch::Patch;
use serde::Deserialize;
use serde_json::{Value, from_value};
use std::{sync::Arc, time::Duration};

use agentc_agent::{
    graph::state::GraphState,
    tools::{
        activity::ActivityDelta,
        errors::ToolError,
        traits::Tool,
        types::{ToolInput, ToolOutput},
    },
    types::{
        capability::{Capability, CapabilitySet},
        tools::ToolDefinition,
    },
};

use crate::javascript::runtime::{
    errors::RuntimeError,
    protocol::{ArgValue, FunctionArgs},
    traits::{Runtime, RuntimeExt},
};

#[derive(Deserialize)]
struct ToolResult {
    output: Value,
    state_update: Option<Patch>,
}

pub struct JavascriptTool {
    pool: Arc<dyn Runtime>,
    export_name: String,
    definition: ToolDefinition,
    capabilities: CapabilitySet,
    timeout: Duration,
}

impl JavascriptTool {
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
    type State = Value;
    type StateUpdate = Patch;

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
        // Weak reference prevents the Arc on the emitter from keeping the sender alive after
        // the drain is dropped, which would cause a deadlock in the react graph call_tools path.
        let emit_tx = Arc::new(input.emitter.and_then(|e| e.sender()));
        let weak_tx = Arc::downgrade(&emit_tx);

        let mut fields: Vec<(String, ArgValue)> = vec![
            ("args".into(), ArgValue::Json(input.args)),
            ("state".into(), ArgValue::Json(input.state.unwrap_or(Value::Null))),
        ];

        if emit_tx.is_some() {
            fields.push((
                "emit".into(),
                ArgValue::Callable(Arc::new(move |params| {
                    if let Some(arc) = weak_tx.upgrade()
                        && let Some(tx) = arc.as_ref()
                    {
                        let delta: ActivityDelta = match params.into_iter().next() {
                            Some(ArgValue::Json(v)) => match from_value(v) {
                                Ok(d) => d,
                                Err(_) => return Ok(Value::Null),
                            },
                            _ => return Ok(Value::Null),
                        };
                        let _ = tx.try_send(delta);
                    }
                    Ok(Value::Null)
                })),
            ));
        }

        let args = FunctionArgs::new().param(ArgValue::Object(fields));

        let result = tokio::time::timeout(
            self.timeout,
            self.pool
                .call_export_method::<ToolResult>(&self.export_name, args),
        )
        .await
        .map_err(|_| {
            ToolError::sourced_execution_error(
                "javascript",
                "tool execution timed out",
                None::<RuntimeError>,
            )
        })?
        .map_err(ToolError::from)?;

        let mut output = ToolOutput::ok(result.output);

        if let Some(patch) = result.state_update {
            output = output.with_state(patch);
        }

        Ok(output)
    }
}

pub struct JavascriptToolBuilder {
    runtime: Option<Arc<dyn Runtime>>,
    export_name: Option<String>,
    capabilities: CapabilitySet,
    timeout: Duration,
}

impl JavascriptToolBuilder {
    pub fn new() -> Self {
        Self {
            runtime: None,
            export_name: None,
            capabilities: CapabilitySet::default(),
            timeout: Duration::from_secs(30),
        }
    }

    pub fn runtime(mut self, runtime: Arc<dyn Runtime>) -> Self {
        self.runtime = Some(runtime);
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

    pub async fn build(self) -> Result<JavascriptTool, RuntimeError> {
        let runtime = self
            .runtime
            .expect("runtime must be provided to build a JavascriptTool");
        let export_name = self
            .export_name
            .expect("export_name must be provided to build a JavascriptTool");

        let definition = runtime
            .get_export::<ToolDefinition>(&export_name)
            .await?;

        Ok(JavascriptTool {
            pool: runtime,
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
    use super::*;
    use agentc_agent::{
        graph::state::{GraphState, GraphStateInput, GraphStateUpdate},
        tools::{
            dispatcher::{DispatchOutcome, ToolRegistryExt},
            registry::ToolRegistry,
            types::ToolExecutionContext,
        },
        types::tools::ToolCall,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use crate::javascript::runtime::quickjs::runtime::QuickJsRuntimeBuilder;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    struct DummyState;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    struct DummyUpdate {
        k: i32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct DummyInput;

    impl GraphState for DummyState {
        type Update = DummyUpdate;
        type Input = DummyInput;
    }

    impl GraphStateUpdate for DummyUpdate {
        type State = DummyState;
        fn apply(self, _: &mut DummyState) {}
        fn merge(self, _other: Self) -> Self {
            self
        }
    }

    impl GraphStateInput for DummyInput {
        type State = DummyState;
        fn initialize(self) -> DummyState {
            DummyState
        }
    }

    // Tools receive a single input object: { args, state, emit? }.
    const JS: &str = r#"
        export const add = {
            name: "add",
            description: "add a and b",
            parameters: {},
            async execute(input) { return { output: input.args.a + input.args.b, state_update: null }; }
        };
        export const double = {
            name: "double",
            description: "double x",
            parameters: {},
            async execute(input) { return { output: input.args.x * 2, state_update: null }; }
        };
    "#;

    #[tokio::test]
    async fn multi_tool_shared_runtime() {
        let runtime: Arc<dyn Runtime> = Arc::new(
            QuickJsRuntimeBuilder::new()
                .source(JS)
                .num_interpreters(1)
                .build()
                .await
                .unwrap(),
        );

        let add = JavascriptToolBuilder::new()
            .runtime(runtime.clone())
            .export_name("add")
            .build()
            .await
            .unwrap();

        let double = JavascriptToolBuilder::new()
            .runtime(runtime.clone())
            .export_name("double")
            .build()
            .await
            .unwrap();

        let dispatcher = ToolRegistry::builder()
            .with_tool::<DummyState, _>(add)
            .with_tool::<DummyState, _>(double)
            .build()
            .dispatcher();

        let state = DummyState;

        let out_add = dispatcher
            .dispatch::<DummyState>(
                ToolCall {
                    id: "1".into(),
                    name: "add".into(),
                    arguments: json!({"a": 3, "b": 4}),
                },
                &state,
                ToolExecutionContext {
                    tenant_id: "test".to_string(),
                    session_id: Default::default(),
                    run_id: Default::default(),
                },
                None,
            )
            .await;

        let out_double = dispatcher
            .dispatch::<DummyState>(
                ToolCall {
                    id: "2".into(),
                    name: "double".into(),
                    arguments: json!({"x": 5}),
                },
                &state,
                ToolExecutionContext {
                    tenant_id: "test".to_string(),
                    session_id: Default::default(),
                    run_id: Default::default(),
                },
                None,
            )
            .await;

        assert!(
            matches!(out_add,    DispatchOutcome::Success { ref content, .. } if *content == json!(7))
        );
        assert!(
            matches!(out_double, DispatchOutcome::Success { ref content, .. } if *content == json!(10))
        );
    }

    // emit is passed as a field inside the input object: input.emit(delta)
    const EMIT_JS: &str = r#"
        export const emitter_tool = {
            name: "emitter_tool",
            description: "emits a delta then returns",
            parameters: {},
            async execute(input) {
                if (input.emit) {
                    input.emit({ activity_type: "test_type", patch: [{"op":"add","path":"/foo","value":1}] });
                }
                return { output: "done", state_update: null };
            }
        };
    "#;

    #[tokio::test]
    async fn emit_delivers_activity_delta() {
        use agentc_agent::tools::activity::{ActivityDelta, ActivityEmitter};
        use tokio::sync::mpsc;

        let runtime: Arc<dyn Runtime> = Arc::new(
            QuickJsRuntimeBuilder::new()
                .source(EMIT_JS)
                .num_interpreters(1)
                .build()
                .await
                .unwrap(),
        );

        let tool = JavascriptToolBuilder::new()
            .runtime(runtime)
            .export_name("emitter_tool")
            .build()
            .await
            .unwrap();

        let dispatcher = ToolRegistry::builder()
            .with_tool::<DummyState, _>(tool)
            .build()
            .dispatcher();

        let state = DummyState;
        let (tx, mut rx) = mpsc::channel::<ActivityDelta>(8);

        let outcome = dispatcher
            .dispatch::<DummyState>(
                ToolCall {
                    id: "1".into(),
                    name: "emitter_tool".into(),
                    arguments: json!({}),
                },
                &state,
                ToolExecutionContext {
                    tenant_id: "test".to_string(),
                    session_id: Default::default(),
                    run_id: Default::default(),
                },
                Some(ActivityEmitter::new(tx)),
            )
            .await;

        assert!(
            matches!(outcome, DispatchOutcome::Success { ref content, .. } if *content == json!("done"))
        );

        let delta = rx
            .recv()
            .await
            .expect("expected a delta");
        assert_eq!(delta.activity_type, "test_type");
        assert_eq!(delta.patch.len(), 1);
    }

    // A tool that guards on input.emit before calling it must succeed without throwing
    // when no ActivityEmitter is supplied.
    #[tokio::test]
    async fn emit_is_absent_without_emitter() {
        let runtime: Arc<dyn Runtime> = Arc::new(
            QuickJsRuntimeBuilder::new()
                .source(EMIT_JS)
                .num_interpreters(1)
                .build()
                .await
                .unwrap(),
        );

        let tool = JavascriptToolBuilder::new()
            .runtime(runtime)
            .export_name("emitter_tool")
            .build()
            .await
            .unwrap();

        let dispatcher = ToolRegistry::builder()
            .with_tool::<DummyState, _>(tool)
            .build()
            .dispatcher();

        let outcome = dispatcher
            .dispatch::<DummyState>(
                ToolCall {
                    id: "1".into(),
                    name: "emitter_tool".into(),
                    arguments: json!({}),
                },
                &DummyState,
                ToolExecutionContext {
                    tenant_id: "test".to_string(),
                    session_id: Default::default(),
                    run_id: Default::default(),
                },
                None,
            )
            .await;

        assert!(
            matches!(outcome, DispatchOutcome::Success { ref content, .. } if *content == json!("done"))
        );
    }

    // A tool may call emit multiple times in a single invocation. Each call should
    // produce a separate ActivityDelta on the receiver in order.
    const MULTI_EMIT_JS: &str = r#"
        export const multi_emitter = {
            name: "multi_emitter",
            description: "emits several deltas",
            parameters: {},
            async execute(input) {
                input.emit({ activity_type: "step_1", patch: [{"op":"add","path":"/a","value":1}] });
                input.emit({ activity_type: "step_2", patch: [{"op":"add","path":"/b","value":2}] });
                input.emit({ activity_type: "step_3", patch: [{"op":"add","path":"/c","value":3}] });
                return { output: "done", state_update: null };
            }
        };
    "#;

    #[tokio::test]
    async fn multiple_emits_are_all_delivered() {
        use agentc_agent::tools::activity::{ActivityDelta, ActivityEmitter};
        use tokio::sync::mpsc;

        let runtime: Arc<dyn Runtime> = Arc::new(
            QuickJsRuntimeBuilder::new()
                .source(MULTI_EMIT_JS)
                .num_interpreters(1)
                .build()
                .await
                .unwrap(),
        );

        let tool = JavascriptToolBuilder::new()
            .runtime(runtime)
            .export_name("multi_emitter")
            .build()
            .await
            .unwrap();

        let dispatcher = ToolRegistry::builder()
            .with_tool::<DummyState, _>(tool)
            .build()
            .dispatcher();

        let (tx, mut rx) = mpsc::channel::<ActivityDelta>(8);

        dispatcher
            .dispatch::<DummyState>(
                ToolCall {
                    id: "1".into(),
                    name: "multi_emitter".into(),
                    arguments: json!({}),
                },
                &DummyState,
                ToolExecutionContext {
                    tenant_id: "test".to_string(),
                    session_id: Default::default(),
                    run_id: Default::default(),
                },
                Some(ActivityEmitter::new(tx)),
            )
            .await;

        let d1 = rx
            .recv()
            .await
            .expect("expected delta 1");
        let d2 = rx
            .recv()
            .await
            .expect("expected delta 2");
        let d3 = rx
            .recv()
            .await
            .expect("expected delta 3");

        assert_eq!(d1.activity_type, "step_1");
        assert_eq!(d2.activity_type, "step_2");
        assert_eq!(d3.activity_type, "step_3");
    }

    // A tool that reads input.state and reflects it in the output proves that state
    // is correctly serialized and passed through to the JS execution context.
    const STATE_READ_JS: &str = r#"
        export const state_reader = {
            name: "state_reader",
            description: "echoes a field from the agent state",
            parameters: {},
            async execute(input) {
                return { output: input.state ? input.state.value : null, state_update: null };
            }
        };
    "#;

    #[tokio::test]
    async fn state_is_passed_to_tool() {
        let runtime: Arc<dyn Runtime> = Arc::new(
            QuickJsRuntimeBuilder::new()
                .source(STATE_READ_JS)
                .num_interpreters(1)
                .build()
                .await
                .unwrap(),
        );

        let tool = JavascriptToolBuilder::new()
            .runtime(runtime)
            .export_name("state_reader")
            .build()
            .await
            .unwrap();

        let dispatcher = ToolRegistry::builder()
            .with_tool::<DummyState, _>(tool)
            .build()
            .dispatcher();

        let outcome = dispatcher
            .dispatch::<DummyState>(
                ToolCall {
                    id: "1".into(),
                    name: "state_reader".into(),
                    arguments: json!({}),
                },
                &DummyState,
                ToolExecutionContext {
                    tenant_id: "test".to_string(),
                    session_id: Default::default(),
                    run_id: Default::default(),
                },
                None,
            )
            .await;

        // DummyState serializes as `{}` so input.state.value is undefined → null output.
        assert!(
            matches!(outcome, DispatchOutcome::Success { ref content, .. } if content.is_null())
        );
    }

    // A tool that returns a state_update JSON Patch. The patch should be propagated
    // through ToolOutput so callers can apply it to the agent state.
    const STATE_UPDATE_JS: &str = r#"
        export const state_patcher = {
            name: "state_patcher",
            description: "returns a state patch",
            parameters: {},
            async execute(input) {
                return {
                    output: "patched",
                    state_update: [{"op":"add","path":"/k","value":99}]
                };
            }
        };
    "#;

    #[tokio::test]
    async fn state_update_propagates() {
        let runtime: Arc<dyn Runtime> = Arc::new(
            QuickJsRuntimeBuilder::new()
                .source(STATE_UPDATE_JS)
                .num_interpreters(1)
                .build()
                .await
                .unwrap(),
        );

        let tool = JavascriptToolBuilder::new()
            .runtime(runtime)
            .export_name("state_patcher")
            .build()
            .await
            .unwrap();

        let dispatcher = ToolRegistry::builder()
            .with_tool::<DummyState, _>(tool)
            .build()
            .dispatcher();

        let outcome = dispatcher
            .dispatch::<DummyState>(
                ToolCall {
                    id: "1".into(),
                    name: "state_patcher".into(),
                    arguments: json!({}),
                },
                &DummyState,
                ToolExecutionContext {
                    tenant_id: "test".to_string(),
                    session_id: Default::default(),
                    run_id: Default::default(),
                },
                None,
            )
            .await;

        assert!(
            matches!(outcome, DispatchOutcome::Success { ref content, .. } if *content == json!("patched"))
        );
    }

    // Building a tool whose export name does not exist in the module should fail at
    // build time rather than at invocation time.
    #[tokio::test]
    async fn builder_rejects_unknown_export() {
        let runtime: Arc<dyn Runtime> = Arc::new(
            QuickJsRuntimeBuilder::new()
                .source(JS)
                .num_interpreters(1)
                .build()
                .await
                .unwrap(),
        );

        let result = JavascriptToolBuilder::new()
            .runtime(runtime)
            .export_name("nonexistent")
            .build()
            .await;

        assert!(result.is_err());
    }
}
