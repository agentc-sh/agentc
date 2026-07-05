// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use json_patch::Patch;
use serde::Deserialize;
use serde_json::{Value, from_value, json};
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

use crate::python::runtime::{ArgValue, FunctionArgs, Runtime, RuntimeExt};

/// The shape of the JSON object that a Python tool's `invoke` method must return.
///
/// The Python `invoke` method should return a dict with an `output` key containing
/// the tool result and an optional `state_update` key containing an RFC 6902 patch.
#[derive(Debug, Deserialize)]
struct ExecuteResult {
    output: Value,
    state_update: Option<Patch>,
}

pub struct PythonTool {
    pool: Arc<dyn Runtime>,
    tool_name: String,
    definition: ToolDefinition,
    capabilities: CapabilitySet,
    timeout: Duration,
}

impl PythonTool {
    pub fn builder() -> PythonToolBuilder {
        PythonToolBuilder::new()
    }
}

#[async_trait]
impl<S> Tool<S> for PythonTool
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
        // We use a weak reference for the emitter in the callback because the keyword_callable method converts
        // to an arc captured that gets passed into the Python runtime and causes the runtime to have a strong
        // reference on the emitter, preventing the drain from being dropped and causing a deadlock in the react
        // graph call_tools method.
        let emit_tx = Arc::new(input.emitter.and_then(|e| e.sender()));
        let weak_tx = Arc::downgrade(&emit_tx);

        let result = self
            .pool
            .call_function_with_timeout::<ExecuteResult>(
                "agentc_tdk",
                "invoke_tool",
                FunctionArgs::new()
                    .positional(json!(self.tool_name))
                    .positional(input.args)
                    .positional(input.state.unwrap_or(Value::Null))
                    .keyword_callable("emit", move |args| {
                        if let Some(arc) = weak_tx.upgrade()
                            && let Some(tx) = arc.as_ref()
                        {
                            let _ = tx.try_send(ActivityDelta {
                                activity_type: match args.positional.first() {
                                    Some(ArgValue::Json(Value::String(s))) => s.clone(),
                                    _ => return Ok(Value::Null),
                                },
                                patch: match args.positional.get(1) {
                                    Some(ArgValue::Json(v)) => {
                                        from_value(v.clone()).unwrap_or_default()
                                    }
                                    _ => vec![],
                                },
                            });
                        }

                        Ok(Value::Null)
                    }),
                self.timeout,
            )
            .await
            .map_err(ToolError::from)?;

        let mut output = ToolOutput::ok(result.output);

        if let Some(patch) = result.state_update {
            output = output.with_state(patch);
        }

        Ok(output)
    }
}

pub struct PythonToolBuilder {
    runtime: Option<Arc<dyn Runtime>>,
    module: Option<String>,
    tool_name: Option<String>,
    capabilities: CapabilitySet,
    timeout: Duration,
}

impl PythonToolBuilder {
    pub fn new() -> Self {
        Self {
            runtime: None,
            module: None,
            tool_name: None,
            capabilities: CapabilitySet::empty(),
            timeout: Duration::from_secs(30),
        }
    }

    pub fn runtime(mut self, runtime: Arc<dyn Runtime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    pub fn module(mut self, module: impl Into<String>) -> Self {
        self.module = Some(module.into());
        self
    }

    pub fn tool_name(mut self, name: impl Into<String>) -> Self {
        self.tool_name = Some(name.into());
        self
    }

    pub fn capability(mut self, cap: impl Into<Capability>) -> Self {
        self.capabilities.insert(cap.into());
        self
    }

    pub fn capabilities<I, C>(mut self, caps: I) -> Self
    where
        I: IntoIterator<Item = C>,
        C: Into<Capability>,
    {
        self.capabilities
            .extend(caps.into_iter().map(Into::into));
        self
    }

    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = duration;
        self
    }

    /// Discover the tool's definition from `agentc_tdk` and build the [`PythonTool`].
    pub async fn build(self) -> Result<PythonTool, ToolError> {
        let runtime = self
            .runtime
            .expect("runtime must be provided");
        let module = self
            .module
            .expect("module must be provided");
        let tool_name = self
            .tool_name
            .expect("tool_name must be provided");

        runtime
            .import_with_timeout(&module, self.timeout)
            .await?;

        let payload = runtime
            .call_function_with_timeout::<Value>(
                "agentc_tdk",
                "get_tool_definition",
                FunctionArgs::new().positional(json!(&tool_name)),
                self.timeout,
            )
            .await
            .map_err(ToolError::from)?;

        Ok(PythonTool {
            pool: runtime,
            tool_name: tool_name.clone(),
            definition: ToolDefinition {
                name: payload["name"]
                    .as_str()
                    .unwrap_or(&tool_name)
                    .to_string(),
                description: payload["description"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                parameters: payload["schema"].clone(),
            },
            capabilities: self.capabilities,
            timeout: self.timeout,
        })
    }
}

impl Default for PythonToolBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "python-embedded"))]
mod tests {
    use super::PythonTool;
    use crate::python::{EmbeddedRuntime, runtime::RuntimeExt};
    use agentc_agent::{
        graph::state::{GraphState, GraphStateInput, GraphStateUpdate},
        tools::{
            activity::{ActivityDelta, ActivityEmitter},
            dispatcher::{DispatchOutcome, ToolRegistryExt},
            registry::ToolRegistry,
        },
        types::tools::ToolCall,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use std::sync::Arc;

    // Minimal concrete state types to satisfy Tool<U> trait bounds.
    // None of these methods are exercised by PythonTool itself.

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

    /// Minimal pure-Python stub that mirrors the `agentc_tdk` public API.
    /// Injected as `agentc_tdk` in sys.modules so tests don't depend on the
    /// external package being installed in the embedded interpreter.
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

    fn tool_runtime() -> Arc<EmbeddedRuntime> {
        Arc::new(
            EmbeddedRuntime::builder()
                .num_interpreters(1)
                .channel_size(32)
                .build()
                .expect("failed to build EmbeddedRuntime"),
        )
    }

    // Registers a Python module by name in sys.modules using exec.
    // The source is transferred via a scope global to avoid string escaping issues.
    async fn inject_module(runtime: &EmbeddedRuntime, name: &str, src: &str) {
        runtime
            .set_global("_inject_src", src)
            .await
            .expect("set_global failed");

        runtime
            .exec(&format!(
                "import sys as _sys, types as _types\n\
                 _m = _types.ModuleType({name:?})\n\
                 exec(_inject_src, _m.__dict__)\n\
                 _sys.modules[{name:?}] = _m"
            ))
            .await
            .expect("module injection failed");
    }

    async fn inject_agentc_tdk(runtime: &EmbeddedRuntime) {
        inject_module(runtime, "agentc_tdk", AGENTC_TDK_STUB).await;
    }

    // Two Python tools that live in the same module and share one runtime.
    // Each routes to its own tool name and returns the correct computed result
    // when dispatched through a ToolRegistry.
    #[tokio::test]
    async fn multi_tool_shared_runtime() {
        let runtime = tool_runtime();
        inject_agentc_tdk(&runtime).await;

        inject_module(
            &runtime,
            "math_tools",
            r#"
from agentc_tdk import Tool, Args, ToolOutput
from dataclasses import dataclass

@dataclass
class AddArgs(Args):
    a: int
    b: int

@dataclass
class DoubleArgs(Args):
    x: int

class AddTool(Tool):
    args = AddArgs
    name = "add"
    description = "add a and b"
    schema = {}

    def execute(self, input):
        return ToolOutput(output=input.args.a + input.args.b)

class DoubleTool(Tool):
    args = DoubleArgs
    name = "double"
    description = "double x"
    schema = {}

    def execute(self, input):
        return ToolOutput(output=input.args.x * 2)
"#,
        )
        .await;

        let add = PythonTool::builder()
            .runtime(runtime.clone())
            .module("math_tools")
            .tool_name("add")
            .build()
            .await
            .unwrap();

        let double = PythonTool::builder()
            .runtime(runtime.clone())
            .module("math_tools")
            .tool_name("double")
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

    // A Python tool that calls emit produces an ActivityDelta on the receiver
    // provided via the ActivityEmitter passed to the dispatcher.
    #[tokio::test]
    async fn emit_delivers_activity_delta() {
        let runtime = tool_runtime();
        inject_agentc_tdk(&runtime).await;

        inject_module(
            &runtime,
            "emit_tool",
            r#"
from agentc_tdk import Tool, Args, ToolOutput
from dataclasses import dataclass

@dataclass
class EmptyArgs(Args): ...

class EmitterTool(Tool):
    args = EmptyArgs
    name = "emitter_tool"
    description = "emits a delta then returns"
    schema = {}

    def execute(self, input):
        if input.emit is not None:
            input.emit("test_type", [{"op": "add", "path": "/foo", "value": 1}])
        return ToolOutput(output="done")
"#,
        )
        .await;

        let tool = PythonTool::builder()
            .runtime(runtime)
            .module("emit_tool")
            .tool_name("emitter_tool")
            .build()
            .await
            .unwrap();

        let dispatcher = ToolRegistry::builder()
            .with_tool::<DummyState, _>(tool)
            .build()
            .dispatcher();

        let state = DummyState;
        let (tx, mut rx) = tokio::sync::mpsc::channel::<ActivityDelta>(8);

        let outcome = dispatcher
            .dispatch::<DummyState>(
                ToolCall {
                    id: "1".into(),
                    name: "emitter_tool".into(),
                    arguments: json!({}),
                },
                &state,
                Some(ActivityEmitter::new(tx)),
            )
            .await;

        assert!(
            matches!(outcome, DispatchOutcome::Success { ref content, .. } if *content == json!("done"))
        );

        let delta = rx
            .recv()
            .await
            .expect("expected an activity delta");
        assert_eq!(delta.activity_type, "test_type");
        assert_eq!(delta.patch.len(), 1);
    }

    // A Python tool that returns a state_update patch propagates it through ToolOutput.
    #[tokio::test]
    async fn state_update_propagates() {
        let runtime = tool_runtime();
        inject_agentc_tdk(&runtime).await;

        inject_module(
            &runtime,
            "state_tool",
            r#"
from agentc_tdk import Tool, Args, ToolOutput
from dataclasses import dataclass

@dataclass
class EmptyArgs(Args): ...

class StateTool(Tool):
    args = EmptyArgs
    name = "state_tool"
    description = "returns a state patch"
    schema = {}

    def execute(self, input):
        return ToolOutput(
            output="patched",
            state_update=[{"op": "add", "path": "/k", "value": 99}],
        )
"#,
        )
        .await;

        let tool = PythonTool::builder()
            .runtime(runtime)
            .module("state_tool")
            .tool_name("state_tool")
            .build()
            .await
            .unwrap();

        let dispatcher = ToolRegistry::builder()
            .with_tool::<DummyState, _>(tool)
            .build()
            .dispatcher();

        let state = DummyState;

        let outcome = dispatcher
            .dispatch::<DummyState>(
                ToolCall {
                    id: "1".into(),
                    name: "state_tool".into(),
                    arguments: json!({}),
                },
                &state,
                None,
            )
            .await;

        assert!(
            matches!(outcome, DispatchOutcome::Success { ref content, .. } if *content == json!("patched"))
        );
    }

    // Builder returns an error when the tool name is not in the registry.
    #[tokio::test]
    async fn builder_rejects_unknown_tool_name() {
        let runtime = tool_runtime();
        inject_agentc_tdk(&runtime).await;

        let result = PythonTool::builder()
            .runtime(runtime)
            .module("nonexistent_module")
            .tool_name("nonexistent_tool")
            .build()
            .await;

        assert!(result.is_err());
    }
}
