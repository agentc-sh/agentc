// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use agentc_agent::types::{capability::CapabilityOverride, tools::ToolDefinition};

use crate::types::{context_var::ContextVar, model::ModelConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub id: Uuid,
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Uuid,
    pub checkpoint_id: Option<Uuid>,
    pub model: Option<ModelConfig>,
    pub capability_override: Option<CapabilityOverride>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub context_vars: Option<Vec<ContextVar>>,
    pub context: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
