// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct InvocationContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    pub attributes: Map<String, Value>,
}

impl InvocationContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_conversation_id(mut self, conversation_id: impl Into<String>) -> Self {
        self.conversation_id = Some(conversation_id.into());
        self
    }

    pub fn with_agent_name(mut self, agent_name: impl Into<String>) -> Self {
        self.agent_name = Some(agent_name.into());
        self
    }

    pub fn with_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn with_attribute(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        self.attributes
            .insert(name.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, to_value};

    use super::*;

    #[test]
    fn serializes_only_present_values() {
        assert_eq!(
            to_value(
                InvocationContext::new()
                    .with_conversation_id("c1")
                    .with_tenant_id("acme")
                    .with_attribute("run_id", "r1")
            )
            .unwrap(),
            json!({
                "conversation_id": "c1",
                "tenant_id": "acme",
                "attributes": { "run_id": "r1" }
            })
        );
    }

    #[test]
    fn empty_context_serializes_empty_attributes() {
        assert_eq!(to_value(InvocationContext::new()).unwrap(), json!({ "attributes": {} }));
    }
}
