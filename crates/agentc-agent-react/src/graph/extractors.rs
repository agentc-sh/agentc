// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{collections::HashSet, sync::Arc};

use agentc_model::traits::CompletionModel;

use agentc_agent::{
    context::AgentContext,
    graph::{
        context::{FromRuntimeContext, RuntimeContext},
        errors::GraphError,
        state::GraphNode,
    },
    tools::dispatcher::{ToolDispatcher, ToolRegistryExt},
    types::tools::ToolDefinition,
};

use crate::{
    graph::state::ReActState,
    types::{context_var::ContextVar, message::Message, model::ModelConfig as ReActModelConfig},
};

/// An extractor for the messages from the agent state.
pub struct Messages(pub Vec<Message>);

impl Messages {
    pub fn as_inner(&self) -> &[Message] {
        &self.0
    }

    pub fn into_inner(self) -> Vec<Message> {
        self.0
    }
}

impl<N> FromRuntimeContext<N> for Messages
where
    N: GraphNode<State = ReActState>,
{
    fn from_rtx(rtx: &RuntimeContext<N>) -> Result<Self, GraphError> {
        Ok(Messages(rtx.state.messages.clone()))
    }
}

/// An extractor for the context variables from the agent state.
pub struct ContextVars(pub Vec<ContextVar>);

impl ContextVars {
    pub fn as_inner(&self) -> &[ContextVar] {
        &self.0
    }

    pub fn into_inner(self) -> Vec<ContextVar> {
        self.0
    }
}

impl<N> FromRuntimeContext<N> for ContextVars
where
    N: GraphNode<State = ReActState>,
{
    fn from_rtx(rtx: &RuntimeContext<N>) -> Result<Self, GraphError> {
        Ok(ContextVars(rtx.state.context_vars.clone()))
    }
}

/// An extractor for the model configuration from the agent state.
pub struct ModelConfig(pub Option<ReActModelConfig>);

impl ModelConfig {
    pub fn as_inner(&self) -> Option<&ReActModelConfig> {
        self.0.as_ref()
    }

    pub fn into_inner(self) -> Option<ReActModelConfig> {
        self.0
    }
}

impl<N> FromRuntimeContext<N> for ModelConfig
where
    N: GraphNode<State = ReActState>,
{
    fn from_rtx(rtx: &RuntimeContext<N>) -> Result<Self, GraphError> {
        Ok(ModelConfig(rtx.state.model.clone()))
    }
}

/// An extractor for the model client from the agent context based on
/// the default model or the override in the state.
pub struct Model(pub Arc<dyn CompletionModel>);

impl Model {
    pub fn as_inner(&self) -> &Arc<dyn CompletionModel> {
        &self.0
    }

    pub fn into_inner(self) -> Arc<dyn CompletionModel> {
        self.0
    }
}

impl<N, E, M> FromRuntimeContext<N> for Model
where
    N: GraphNode<Context = AgentContext<E, M>, State = ReActState>,
    E: Send + Clone + 'static,
    M: Send + Clone + 'static,
{
    fn from_rtx(rtx: &RuntimeContext<N>) -> Result<Self, GraphError> {
        let provider = rtx
            .state
            .model
            .as_ref()
            .and_then(|config| config.r#override.as_ref())
            .and_then(|config| config.provider.clone())
            .unwrap_or_else(|| rtx.ctx.identity.provider.clone());

        let model_name = rtx
            .state
            .model
            .as_ref()
            .and_then(|config| config.r#override.as_ref())
            .and_then(|config| config.model.clone())
            .unwrap_or_else(|| rtx.ctx.identity.model.clone());

        match rtx
            .ctx
            .model_registry
            .provider(provider)
            .model(model_name)
        {
            Ok(model) => Ok(Model(model)),
            Err(e) => Err(GraphError::execution_error(e)),
        }
    }
}

/// An extractor for the client specified tools from the agent state.
pub struct ClientTools(pub Vec<ToolDefinition>);

impl ClientTools {
    pub fn as_inner(&self) -> &[ToolDefinition] {
        &self.0
    }

    pub fn into_inner(self) -> Vec<ToolDefinition> {
        self.0
    }
}

impl<N> FromRuntimeContext<N> for ClientTools
where
    N: GraphNode<State = ReActState>,
{
    fn from_rtx(rtx: &RuntimeContext<N>) -> Result<Self, GraphError> {
        Ok(ClientTools(rtx.state.tools.clone()))
    }
}

/// An extractor for the tool dispatcher from the agent context based on
/// the tools specified in the state.
pub struct Tools(pub ToolDispatcher);

impl Tools {
    pub fn as_inner(&self) -> &ToolDispatcher {
        &self.0
    }

    pub fn into_inner(self) -> ToolDispatcher {
        self.0
    }
}

impl<N, E, M> FromRuntimeContext<N> for Tools
where
    N: GraphNode<Context = AgentContext<E, M>, State = ReActState>,
    E: Send + Clone + 'static,
    M: Send + Clone + 'static,
{
    fn from_rtx(rtx: &RuntimeContext<N>) -> Result<Self, GraphError> {
        Ok(Tools(
            rtx.ctx
                .tool_registry
                .dispatcher()
                .with_client_tools(
                    ClientTools::from_rtx(rtx)?
                        .into_inner()
                        .into_iter()
                        .map(|t| t.name.clone())
                        .collect::<Vec<_>>(),
                ),
        ))
    }
}

/// An extractor for the tool definitions from the agent context.
pub struct ToolDefinitions(pub Vec<ToolDefinition>);

impl ToolDefinitions {
    pub fn as_inner(&self) -> &[ToolDefinition] {
        &self.0
    }

    pub fn into_inner(self) -> Vec<ToolDefinition> {
        self.0
    }
}

impl<N, E, M> FromRuntimeContext<N> for ToolDefinitions
where
    N: GraphNode<Context = AgentContext<E, M>, State = ReActState>,
    E: Send + Clone + 'static,
    M: Send + Clone + 'static,
{
    fn from_rtx(rtx: &RuntimeContext<N>) -> Result<Self, GraphError> {
        let effective_capabilities = rtx
            .ctx
            .identity
            .capability_policy
            .effective(
                &rtx.ctx.identity.capabilities,
                &rtx.state
                    .capability_override
                    .clone()
                    .unwrap_or_default(),
            )?;

        let mut definitions = ClientTools::from_rtx(rtx)?.into_inner();

        let mut seen = definitions
            .iter()
            .map(|d| d.name.clone())
            .collect::<HashSet<_>>();

        definitions.extend(
            rtx.ctx
                .tool_registry
                .tools()
                .filter(|tool| {
                    let required = tool.capabilities();
                    required.is_empty() || effective_capabilities.satisfies_all(&required)
                })
                .filter(|tool| seen.insert(tool.definition().name))
                .map(|tool| tool.definition()),
        );

        Ok(ToolDefinitions(definitions))
    }
}
