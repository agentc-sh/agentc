// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::graph::checkpoint::types::RunStatus;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentEvent<S> {
    /// Event indicating that a run has started.
    RunStarted {
        timestamp: f64,
        session_id: Uuid,
        run_id: Uuid,
    },
    /// Event indicating that a run has finished.
    RunFinished {
        timestamp: f64,
        session_id: Uuid,
        run_id: Uuid,
        status: RunStatus,
        #[serde(skip_serializing_if = "Option::is_none")]
        interrupt_payload: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<S>,
    },
    /// Event indicating that a run has encountered an error.
    RunError {
        timestamp: f64,
        session_id: Uuid,
        run_id: Uuid,
        error: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<String>,
    },
}

impl<S> AgentEvent<S> {
    pub fn run_started(session_id: impl Into<Uuid>, run_id: impl Into<Uuid>) -> Self {
        AgentEvent::RunStarted {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            session_id: session_id.into(),
            run_id: run_id.into(),
        }
    }

    pub fn run_finished(
        session_id: impl Into<Uuid>,
        run_id: impl Into<Uuid>,
        status: RunStatus,
        interrupt_payload: Option<Value>,
        result: Option<S>,
    ) -> Self {
        AgentEvent::RunFinished {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            session_id: session_id.into(),
            run_id: run_id.into(),
            status,
            result,
            interrupt_payload,
        }
    }

    pub fn run_error(
        session_id: impl Into<Uuid>,
        run_id: impl Into<Uuid>,
        error: impl Into<String>,
        code: Option<String>,
    ) -> Self {
        AgentEvent::RunError {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            session_id: session_id.into(),
            run_id: run_id.into(),
            error: error.into(),
            code,
        }
    }
}
