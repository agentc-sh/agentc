// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use agentc_agent::tools::activity::{ActivityDelta, ActivityEmitter};
use agentc_executor_typescript::guestjs::host::object::HostObject;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::javascript::types::{JavascriptActivityDelta, JavascriptValue};

pub(crate) struct JavascriptToolInput {
    object: HostObject,
    emitter: Arc<Option<mpsc::Sender<ActivityDelta>>>,
}

impl JavascriptToolInput {
    pub(crate) fn new(args: Value, state: Option<Value>, emitter: Option<ActivityEmitter>) -> Self {
        let emitter = Arc::new(emitter.and_then(|emitter| emitter.sender()));
        let weak_emitter = Arc::downgrade(&emitter);
        let has_emitter = emitter.is_some();

        Self {
            object: HostObject::build(move |input| {
                input
                    .constant("args", JavascriptValue::new(args))
                    .constant("state", JavascriptValue::new(state.unwrap_or(Value::Null)));

                if has_emitter {
                    input.function("emit", move |scope, args| {
                        if let Some(emitter) = weak_emitter.upgrade()
                            && let Some(sender) = emitter.as_ref()
                        {
                            let _ = sender.try_send(
                                args.get::<JavascriptActivityDelta>(scope, 0)?
                                    .into(),
                            );
                        }

                        Ok(())
                    });
                }
            }),
            emitter,
        }
    }

    pub(crate) fn into_parts(self) -> (HostObject, Arc<Option<mpsc::Sender<ActivityDelta>>>) {
        (self.object, self.emitter)
    }
}

#[cfg(test)]
mod tests {
    use agentc_agent::tools::activity::{ActivityDelta, ActivityEmitter};
    use agentc_executor_typescript::executor::Executor;
    use serde_json::json;
    use tokio::sync::mpsc;

    use crate::javascript::input::JavascriptToolInput;

    const INPUT_SOURCE: &str = r#"
export function inspect(input) {
    input.emit({
        activity_type: "input",
        patch: [],
    });

    return [
        input.args.value,
        input.state.status,
    ].join(":");
}
"#;

    #[tokio::test]
    async fn builds_guest_input_with_activity_emitter() {
        let executor = Executor::builder("input.ts", INPUT_SOURCE)
            .workers(1)
            .build()
            .await
            .unwrap();
        let (sender, mut receiver) = mpsc::channel::<ActivityDelta>(1);

        assert_eq!(
            executor
                .execute(move |context| Box::pin(async move {
                    let (input, _emitter) = JavascriptToolInput::new(
                        json!({"value": 42}),
                        Some(json!({"status": "ready"})),
                        Some(ActivityEmitter::new(sender)),
                    )
                    .into_parts();

                    context
                        .module()
                        .function("inspect")
                        .await?
                        .call::<_, String>((input,))
                        .await
                }))
                .await
                .unwrap(),
            "42:ready",
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
