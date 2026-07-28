// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

use crate::types::capability::{CapabilityPolicy, CapabilitySet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentIdentity {
    /// The name of the agent.
    pub name: String,
    /// The default provider to use for this agent if not specified in the input.
    pub provider: String,
    /// The default model to use for this agent if not specified in the input.
    pub model: String,
    /// The capabilities that this agent has been granted, which determine what tools it can use.
    pub capabilities: CapabilitySet,
    /// The capability policy that determines how to handle custom overrides of capabilities in the input.
    pub capability_policy: CapabilityPolicy,
}
