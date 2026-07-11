// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_agent::{
    tools::{
        errors::ToolError,
        types::ToolExecutionContext,
    },
    types::capability::{
        Capability,
        CapabilitySet,
    },
};

use crate::{
    client::A2aClient,
    tools::tool::{
        A2aCancelTaskTool,
        A2aGetTaskTool,
        A2aSendTaskTool,
        A2aStreamTaskTool,
    },
};

#[derive(Debug, thiserror::Error)]
pub enum A2aToolConfigError {
    #[error("missing A2A tool target field: {0}")]
    MissingField(&'static str),

    #[error("invalid A2A tool target id: {0}")]
    InvalidTargetId(String),

    #[error("missing A2A client for target: {0}")]
    MissingClient(String),
}

#[derive(Debug, Clone)]
pub enum A2aTenantPolicy {
    Fixed(String),
    None,
    Inherit,
}

impl A2aTenantPolicy {
    pub fn resolve(&self, context: &ToolExecutionContext) -> Option<String> {
        match self {
            Self::Fixed(value) => Some(value.clone()),
            Self::None => None,
            Self::Inherit => Some(context.tenant_id.clone()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct A2aToolTarget {
    pub id: String,
    pub name: String,
    pub client: A2aClient,
    pub tenant_policy: A2aTenantPolicy,
    pub capabilities: CapabilitySet,
    pub default_accepted_output_modes: Option<Vec<String>>,
}

impl A2aToolTarget {
    pub fn builder() -> A2aToolTargetBuilder {
        A2aToolTargetBuilder::default()
    }

    pub fn send_task_tool(&self) -> A2aSendTaskTool {
        A2aSendTaskTool::new(self.clone())
    }

    pub fn stream_task_tool(&self) -> A2aStreamTaskTool {
        A2aStreamTaskTool::new(self.clone())
    }

    pub fn get_task_tool(&self) -> A2aGetTaskTool {
        A2aGetTaskTool::new(self.clone())
    }

    pub fn cancel_task_tool(&self) -> A2aCancelTaskTool {
        A2aCancelTaskTool::new(self.clone())
    }

    pub(crate) fn tool_name(&self, operation: &str) -> String {
        format!("a2a_{}_{}", self.id, operation)
    }

    pub(crate) fn operation_error(
        &self,
        operation: &str,
        message: impl Into<String>,
    ) -> ToolError {
        ToolError::execution_error(
            self.tool_name(operation),
            format!(
                "A2A {operation} failed for target '{}': {}",
                self.id,
                message.into(),
            ),
        )
    }
}

#[derive(Debug, Clone)]
pub struct A2aToolTargetBuilder {
    id: Option<String>,
    name: Option<String>,
    client: Option<A2aClient>,
    tenant_policy: A2aTenantPolicy,
    capabilities: CapabilitySet,
    default_accepted_output_modes: Option<Vec<String>>,
}

impl Default for A2aToolTargetBuilder {
    fn default() -> Self {
        Self {
            id: None,
            name: None,
            client: None,
            tenant_policy: A2aTenantPolicy::Inherit,
            capabilities: CapabilitySet::empty(),
            default_accepted_output_modes: None,
        }
    }
}

impl A2aToolTargetBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn client(mut self, client: A2aClient) -> Self {
        self.client = Some(client);
        self
    }

    pub fn tenant_policy(mut self, tenant_policy: A2aTenantPolicy) -> Self {
        self.tenant_policy = tenant_policy;
        self
    }

    pub fn fixed_tenant(mut self, tenant: impl Into<String>) -> Self {
        self.tenant_policy = A2aTenantPolicy::Fixed(tenant.into());
        self
    }

    pub fn no_tenant(mut self) -> Self {
        self.tenant_policy = A2aTenantPolicy::None;
        self
    }

    pub fn inherit_tenant(mut self) -> Self {
        self.tenant_policy = A2aTenantPolicy::Inherit;
        self
    }

    pub fn capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities
            .insert(Capability::new(capability.into()));
        self
    }

    pub fn capabilities<I, C>(mut self, capabilities: I) -> Self
    where
        I: IntoIterator<Item = C>,
        C: Into<Capability>,
    {
        self.capabilities.extend(capabilities);
        self
    }

    pub fn default_accepted_output_modes<I, M>(mut self, modes: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Into<String>,
    {
        self.default_accepted_output_modes = Some(
            modes
                .into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    pub fn build(self) -> Result<A2aToolTarget, A2aToolConfigError> {
        let id = self
            .id
            .ok_or(A2aToolConfigError::MissingField("id"))?;

        if id.is_empty()
            || !id
                .chars()
                .all(|value| {
                    value.is_ascii_lowercase()
                        || value.is_ascii_digit()
                        || value == '_'
                })
        {
            return Err(A2aToolConfigError::InvalidTargetId(id));
        }

        Ok(A2aToolTarget {
            name: self
                .name
                .unwrap_or_else(|| id.clone()),
            client: self
                .client
                .ok_or_else(|| A2aToolConfigError::MissingClient(id.clone()))?,
            id,
            tenant_policy: self.tenant_policy,
            capabilities: self.capabilities,
            default_accepted_output_modes: self.default_accepted_output_modes,
        })
    }
}
