// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use agentc_agent::{
    tools::activity::{ActivityDelta, ActivityEmitter},
    types::tools::ToolDefinition,
};
use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{
        FromGuest,
        host::function::HostFn,
        marshal::serde::Serde,
    },
};
use json_patch::{Patch, PatchOperation};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;

#[derive(Deserialize, FromGuest)]
#[guestpy(crate_path = agentc_executor_python::guestpy)]
pub(crate) struct PythonToolDefinition {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    schema: Value,
}

impl PythonToolDefinition {
    pub(crate) fn into_definition(self, tool_name: &str) -> ToolDefinition {
        ToolDefinition {
            name: self.name.unwrap_or_else(|| tool_name.to_string()),
            description: self.description.unwrap_or_default(),
            parameters: self.schema,
        }
    }
}

pub(crate) struct PythonToolInput<B: ExecutorBackend> {
    positional: Vec<Serde<Value>>,
    keyword: Vec<(String, HostFn<B>)>,
    emitter: Arc<Option<mpsc::Sender<ActivityDelta>>>,
}

impl<B: ExecutorBackend> PythonToolInput<B> {
    pub(crate) fn new(
        tool_name: &str,
        args: Value,
        state: Option<Value>,
        emitter: Option<ActivityEmitter>,
    ) -> Self {
        // The guest can retain `emit` past the invocation, so the callable holds only a weak
        // reference. A strong capture would keep the activity channel alive and deadlock the
        // graph-side drain.
        let emitter = Arc::new(emitter.and_then(|emitter| emitter.sender()));
        let weak_emitter = Arc::downgrade(&emitter);
        let has_emitter = emitter.is_some();

        Self {
            positional: vec![
                Serde(json!(tool_name)),
                Serde(args),
                Serde(state.unwrap_or(Value::Null)),
            ],
            keyword: has_emitter
                .then(|| {
                    (
                        "emit".to_string(),
                        HostFn::new(move |enter, args| {
                            if let Some(emitter) = weak_emitter.upgrade()
                                && let Some(sender) = emitter.as_ref()
                                && let Some(activity_type) =
                                    args.optional_positional::<String>(enter, 0)?
                            {
                                let _ = sender.try_send(ActivityDelta {
                                    activity_type,
                                    patch: args
                                        .optional_positional::<Serde<Vec<PatchOperation>>>(
                                            enter, 1,
                                        )?
                                        .unwrap_or_default(),
                                });
                            }

                            Ok(())
                        }),
                    )
                })
                .into_iter()
                .collect(),
            emitter,
        }
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        Vec<Serde<Value>>,
        Vec<(String, HostFn<B>)>,
        Arc<Option<mpsc::Sender<ActivityDelta>>>,
    ) {
        (self.positional, self.keyword, self.emitter)
    }
}

#[derive(Deserialize, FromGuest)]
#[guestpy(crate_path = agentc_executor_python::guestpy)]
pub(crate) struct PythonToolResult {
    pub(crate) output: Value,
    pub(crate) state_update: Option<Patch>,
}

#[cfg(all(test, feature = "python-rustpython"))]
mod tests {
    use agentc_agent::tools::activity::{ActivityDelta, ActivityEmitter};
    use agentc_executor_python::{
        executor::Executor,
        guestpy::{bundle::Bundle, rustpython::RustPython},
    };
    use serde_json::json;
    use tokio::sync::mpsc;

    use crate::python::types::PythonToolInput;

    const INPUT_SOURCE: &str = r#"
def inspect(name, args, state, emit=None):
    emit("input", [])
    return ":".join([name, str(args["value"]), state["status"]])
"#;

    #[tokio::test]
    async fn builds_guest_input_with_activity_emitter() {
        let executor = Executor::<RustPython>::builder("input_fixture")
            .bundle(Bundle::single("input_fixture", INPUT_SOURCE).unwrap())
            .workers(1)
            .build()
            .await
            .unwrap();
        let (sender, mut receiver) = mpsc::channel::<ActivityDelta>(1);

        assert_eq!(
            executor
                .execute(move |context| {
                    Box::pin(async move {
                        let (positional, keyword, _emitter) = PythonToolInput::new(
                            "inspect",
                            json!({"value": 42}),
                            Some(json!({"status": "ready"})),
                            Some(ActivityEmitter::new(sender)),
                        )
                        .into_parts();

                        context
                            .module()
                            .function("inspect")?
                            .call_with::<_, _, String>(positional, keyword)
                    })
                })
                .await
                .unwrap(),
            "inspect:42:ready",
        );
        assert_eq!(
            receiver
                .recv()
                .await
                .unwrap()
                .activity_type,
            "input",
        );

        executor.shutdown().await.unwrap();
    }
}
