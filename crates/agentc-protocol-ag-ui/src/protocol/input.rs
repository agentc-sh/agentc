// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::protocol::{
    context::Context,
    ids::{RunId, ThreadId},
    message::Message,
    tool::Tool,
};

/// Input for running an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RunAgentInput<StateT = Value, FwdPropsT = Value> {
    #[serde(rename = "threadId", default = "ThreadId::random")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default = "RunId::random")]
    pub run_id: RunId,
    #[serde(default)]
    pub state: StateT,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub tools: Vec<Tool>,
    #[serde(default)]
    pub context: Vec<Context>,
    #[serde(rename = "forwardedProps", default)]
    pub forwarded_props: FwdPropsT,
}

impl<StateT, FwdPropsT> RunAgentInput<StateT, FwdPropsT> {
    pub fn new(
        thread_id: impl Into<ThreadId>,
        run_id: impl Into<RunId>,
        state: StateT,
        messages: Vec<Message>,
        tools: Vec<Tool>,
        context: Vec<Context>,
        forwarded_props: FwdPropsT,
    ) -> Self {
        Self {
            thread_id: thread_id.into(),
            run_id: run_id.into(),
            state,
            messages,
            tools,
            context,
            forwarded_props,
        }
    }
}
