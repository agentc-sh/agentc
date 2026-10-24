// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{marker::PhantomData, sync::{Arc, Weak}};

use agentc_agent::tools::activity::{ActivityDelta, ActivityEmitter};
use agentc_executor_typescript::{
    backend::ExecutorBackend,
    guestjs::{
        FromGuest, host_class,
        backend::Backend,
        errors::Error,
        host::function::FunctionDeclaration,
        marshal::nullish::Nullish,
    },
    json::Json,
};
use serde_json::Value;

#[derive(serde::Deserialize, FromGuest)]
#[guestjs(crate_path = agentc_executor_typescript::guestjs)]
#[serde(transparent)]
pub(crate) struct GuestActivityDelta(ActivityDelta);

impl From<GuestActivityDelta> for ActivityDelta {
    fn from(delta: GuestActivityDelta) -> Self {
        delta.0
    }
}

pub struct ToolInput<B: Backend> {
    args: Value,
    state: Value,
    emitter: Option<Weak<ActivityEmitter>>,
    backend: PhantomData<fn() -> B>,
}

#[host_class(backend = B, crate_path = agentc_executor_typescript::guestjs)]
impl<B: ExecutorBackend> ToolInput<B> {
    pub(crate) fn new(
        args: Value,
        state: Value,
        emitter: Option<Arc<ActivityEmitter>>,
    ) -> (Self, Option<Arc<ActivityEmitter>>) {
        (
            Self {
                args,
                state,
                emitter: emitter.as_ref().map(Arc::downgrade),
                backend: PhantomData,
            },
            emitter,
        )
    }

    #[guestjs(get)]
    fn args(&self) -> Result<Json, Error> {
        Ok(Json(self.args.clone()))
    }

    #[guestjs(get)]
    fn state(&self) -> Result<Json, Error> {
        Ok(Json(self.state.clone()))
    }

    #[guestjs(get)]
    fn emit(&self) -> Result<Nullish<FunctionDeclaration<B>>, Error> {
        let Some(emitter) = self.emitter.clone() else {
            return Ok(Nullish::Undefined);
        };

        Ok(Nullish::Some(FunctionDeclaration::new(move |context, args| {
            if let Some(sender) = emitter
                .upgrade()
                .and_then(|emitter| emitter.sender())
            {
                let _ = sender.try_send(
                    args.next::<GuestActivityDelta>(context.enter())?
                        .into(),
                );
            }

            Ok(())
        })))
    }
}

#[cfg(test)]
mod tests {
    use agentc_executor_typescript::{
        executor::Executor,
        guestjs::{bundle::Bundle, handle::protocol::ObjectProtocol, quickjs::QuickJs},
    };
    use tokio::sync::mpsc;

    use super::*;
    use crate::javascript::bindings::{
        executor::ExecutorBuilderToolsExt, guest::GuestTool, output::ToolOutput,
    };

    const TOOL_SOURCE: &str = r#"
import { Tool, ToolInput } from "agentc:tools";

export class Echo extends Tool {
    async execute(input) {
        input.emit?.({ activity_type: "first", patch: [] });
        input.emit?.({ activity_type: "second", patch: [] });

        return {
            output: {
                args: input.args,
                state: input.state,
                emits: typeof input.emit,
            },
        };
    }
}

export class Forger extends Tool {
    async execute() {
        new ToolInput();

        return { output: null };
    }
}
"#;

    async fn invoke(
        emitter: Option<ActivityEmitter>,
        args: Value,
        state: Value,
    ) -> Result<ToolOutput, agentc_executor_typescript::errors::Error> {
        let executor = Executor::<QuickJs>::builder("echo")
            .bundle(Bundle::single("echo.ts", TOOL_SOURCE).unwrap())
            .workers(1)
            .standard_environment()
            .with_tools()
            .build()
            .await
            .expect("executor builds");

        let output = executor
            .execute(move |context| {
                Box::pin(async move {
                    let (guest_input, _guard) = ToolInput::new(args, state, emitter.map(Arc::new));

                    context
                        .module()
                        .class("Echo")?
                        .into_result::<GuestTool<QuickJs>>()
                        .construct(())?
                        .execute(guest_input)?
                        .await
                })
            })
            .await;

        executor
            .shutdown()
            .await
            .expect("executor shuts down");

        output
    }

    #[tokio::test]
    async fn emit_delivers_deltas_in_order_and_closes_with_the_guard() {
        let (sender, mut receiver) = mpsc::channel(8);

        assert_eq!(
            invoke(
                Some(ActivityEmitter::new(sender)),
                serde_json::json!({ "city": "Berlin" }),
                serde_json::json!({ "seen": 1 }),
            )
            .await
            .expect("the tool succeeds")
            .output,
            serde_json::json!({
                "args": { "city": "Berlin" },
                "state": { "seen": 1 },
                "emits": "function",
            }),
        );

        assert_eq!(
            receiver
                .recv()
                .await
                .expect("first delta")
                .activity_type,
            "first",
        );
        assert_eq!(
            receiver
                .recv()
                .await
                .expect("second delta")
                .activity_type,
            "second",
        );
        assert!(
            receiver.recv().await.is_none(),
            "dropping the guard must close the channel, or the graph-side drain never terminates",
        );
    }

    #[tokio::test]
    async fn emit_is_undefined_without_an_emitter() {
        assert_eq!(
            invoke(None, Value::Null, Value::Null)
                .await
                .expect("the tool succeeds")
                .output["emits"],
            "undefined",
        );
    }

    #[tokio::test]
    async fn guest_code_cannot_construct_a_tool_input() {
        let executor = Executor::<QuickJs>::builder("echo")
            .bundle(Bundle::single("echo.ts", TOOL_SOURCE).unwrap())
            .workers(1)
            .standard_environment()
            .with_tools()
            .build()
            .await
            .expect("executor builds");

        let error = match executor
            .execute(|context| {
                Box::pin(async move {
                    let (guest_input, _guard) = ToolInput::new(Value::Null, Value::Null, None);

                    context
                        .module()
                        .class("Forger")?
                        .into_result::<GuestTool<QuickJs>>()
                        .construct(())?
                        .execute(guest_input)?
                        .await
                })
            })
            .await
        {
            Ok(_) => panic!("a host class with no declared constructor accepts construction"),
            Err(error) => error,
        };

        drop(error);

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }
}
