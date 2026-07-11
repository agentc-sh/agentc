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
