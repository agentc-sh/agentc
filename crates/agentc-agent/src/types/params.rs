// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunParams<I> {
    pub input: I,
    pub tenant_id: String,
    pub run_id: Uuid,
    pub session_id: Uuid,
    pub checkpoint_id: Option<Uuid>,
    pub resume_payload: Option<Value>,
}

impl<I> RunParams<I> {
    pub fn new(input: I, tenant_id: impl Into<String>) -> Self {
        Self {
            input,
            tenant_id: tenant_id.into(),
            run_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            checkpoint_id: None,
            resume_payload: None,
        }
    }

    pub fn with_run_id(mut self, run_id: impl Into<Uuid>) -> Self {
        self.run_id = run_id.into();
        self
    }

    pub fn with_session_id(mut self, session_id: impl Into<Uuid>) -> Self {
        self.session_id = session_id.into();
        self
    }

    pub fn with_checkpoint_id(mut self, checkpoint_id: impl Into<Uuid>) -> Self {
        self.checkpoint_id = Some(checkpoint_id.into());
        self
    }

    pub fn with_resume_payload(mut self, resume_payload: impl Into<Value>) -> Self {
        self.resume_payload = Some(resume_payload.into());
        self
    }
}
