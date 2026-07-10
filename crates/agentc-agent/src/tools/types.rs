// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::Serialize;
use serde_json::Value;
use std::{
    error::Error,
    fmt::{Debug, Formatter, Result as FmtResult},
};
use uuid::Uuid;

use crate::{graph::state::AnyStateUpdate, tools::activity::ActivityEmitter};

#[derive(Debug)]
pub struct ToolResult {
    pub id: String,
    pub output: ToolResponse,
}

pub enum ToolResponse {
    Success {
        content: Value,
        state_update: Option<Box<dyn AnyStateUpdate>>,
    },
    Error {
        message: String,
    },
}

impl ToolResponse {
    pub fn ok<S>(value: S) -> Self
    where
        S: Serialize,
    {
        Self::Success {
            content: serde_json::to_value(value).unwrap_or(Value::Null),
            state_update: None,
        }
    }

    pub fn ok_with_state<V, S>(value: V, state_update: S) -> Self
    where
        V: Serialize,
        S: AnyStateUpdate + 'static,
    {
        Self::Success {
            content: serde_json::to_value(value).unwrap_or(Value::Null),
            state_update: Some(Box::new(state_update)),
        }
    }

    pub fn err<E>(err: E) -> Self
    where
        E: Error,
    {
        Self::Error { message: err.to_string() }
    }

    pub fn err_message(message: impl Into<String>) -> Self {
        Self::Error { message: message.into() }
    }
}

impl Debug for ToolResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Success { content, state_update: _ } => f
                .debug_struct("ToolResponse::Success")
                .field("content", content)
                .finish(),
            Self::Error { message } => f
                .debug_struct("ToolResponse::Error")
                .field("message", message)
                .finish(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolExecutionContext {
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Uuid,
}

#[derive(Debug)]
pub struct ToolInput<S = ()> {
    pub args: Value,
    pub context: ToolExecutionContext,
    pub emitter: Option<ActivityEmitter>,
    pub state: Option<S>,
}

impl<S> ToolInput<S> {
    pub fn new(args: Value, context: ToolExecutionContext) -> Self {
        Self {
            args,
            context,
            emitter: None,
            state: None,
        }
    }

    pub fn with_activity_emitter(mut self, emitter: ActivityEmitter) -> Self {
        self.emitter = Some(emitter);
        self
    }

    pub fn maybe_with_activity_emitter(mut self, emitter: Option<ActivityEmitter>) -> Self {
        self.emitter = emitter;
        self
    }

    pub fn with_state(mut self, state: S) -> Self {
        self.state = Some(state);
        self
    }

    pub fn maybe_with_state(mut self, state: Option<S>) -> Self {
        self.state = state;
        self
    }
}

#[derive(Debug)]
pub struct TypedToolInput<I, S = ()> {
    pub args: I,
    pub context: ToolExecutionContext,
    pub emitter: Option<ActivityEmitter>,
    pub state: Option<S>,
}

impl<I, S> TypedToolInput<I, S> {
    pub fn new(args: I, context: ToolExecutionContext) -> Self {
        Self {
            args,
            context,
            emitter: None,
            state: None,
        }
    }

    pub fn with_activity_emitter(mut self, emitter: ActivityEmitter) -> Self {
        self.emitter = Some(emitter);
        self
    }

    pub fn maybe_with_activity_emitter(mut self, emitter: Option<ActivityEmitter>) -> Self {
        self.emitter = emitter;
        self
    }

    pub fn with_state(mut self, state: S) -> Self {
        self.state = Some(state);
        self
    }

    pub fn maybe_with_state(mut self, state: Option<S>) -> Self {
        self.state = state;
        self
    }
}

#[derive(Debug)]
pub struct ToolOutput<U = ()> {
    pub output: Value,
    pub state_update: Option<U>,
}

impl<U> ToolOutput<U>
where
    U: Clone,
{
    pub fn ok(output: Value) -> Self {
        Self { output, state_update: None }
    }

    pub fn with_state(mut self, update: U) -> Self {
        self.state_update = Some(update);
        self
    }
}

pub struct TypedToolOutput<O, U = ()> {
    pub output: O,
    pub state_update: Option<U>,
}

impl<O, U> TypedToolOutput<O, U>
where
    O: Serialize,
    U: Clone,
{
    pub fn ok(output: O) -> Self {
        Self { output, state_update: None }
    }

    pub fn with_state(mut self, update: U) -> Self {
        self.state_update = Some(update);
        self
    }
}
