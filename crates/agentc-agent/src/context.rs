// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::fmt;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use agentc_model::registry::ModelRegistry;
use agentc_prompt::{
    compaction::CompactionStrategy, counter::TokenCounter, env::PromptEnv, source::PromptSource,
    vars::TemplateVars,
};

use crate::{
    graph::{
        context::{FromRuntimeContext, RuntimeContext},
        errors::GraphError,
        state::{GraphContext, GraphNode},
    },
    stream::EventEmitter,
    tools::registry::ToolRegistry,
    types::identity::AgentIdentity,
};

/// The context available to the agent during its execution.
///
/// The `M` parameter is the message type used by the compaction strategy. It
/// defaults to `()`, which pairs with the `NoCompaction` strategy for graphs
/// that do not need context-window management.
#[derive(Clone)]
pub struct AgentContext<E: Send + Clone + 'static, M: Send + Clone + 'static = ()> {
    /// An event emitter for sending events during agent execution.
    pub emitter: EventEmitter<E>,
    /// The model registry for making LLM calls during agent execution.
    pub model_registry: ModelRegistry,
    /// The tool registry for backend tool calls during agent execution.
    pub tool_registry: ToolRegistry,
    /// The agent's identity information, including default provider, model, and prompt template.
    pub identity: AgentIdentity,
    /// The Jinja2 rendering environment shared across all prompt renders.
    pub prompt_env: PromptEnv,
    /// The source that resolves the agent's prompt template before each model call.
    pub prompt_source: Arc<dyn PromptSource>,
    /// The token counter used for budget tracking and compaction decisions.
    pub token_counter: Arc<dyn TokenCounter>,
    /// The compaction strategy applied to the message buffer before each model call.
    pub compaction_strategy: Arc<dyn CompactionStrategy<M>>,
    /// The current Session ID for the agent's execution.
    pub session_id: Uuid,
    /// The current Run ID for the agent's execution.
    pub run_id: Uuid,
    /// The tenant ID for the agent's execution.
    pub tenant_id: String,
    /// Contributors that provide additional variables into the prompt template
    /// render context. Iterated and merged in the model call node before rendering.
    pub template_vars: Vec<Arc<dyn TemplateVars>>,
}

impl<E: Send + Clone + 'static, M: Send + Clone + 'static> fmt::Debug for AgentContext<E, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AgentContext")
            .field("identity", &self.identity)
            .field("session_id", &self.session_id)
            .field("run_id", &self.run_id)
            .field("tenant_id", &self.tenant_id)
            .finish_non_exhaustive()
    }
}

impl<E: Send + Clone + 'static, M: Send + Clone + 'static> AgentContext<E, M> {
    pub fn emit(&self, event: E) -> Result<(), mpsc::error::SendError<E>> {
        self.emitter.emit(event)
    }
}

impl<E: Send + Clone + 'static, M: Send + Clone + 'static> GraphContext for AgentContext<E, M> {}

/// An extractor for the current run's ID from the agent state.
pub struct RunId(pub Uuid);

impl RunId {
    pub fn as_inner(&self) -> &Uuid {
        &self.0
    }

    pub fn into_inner(self) -> Uuid {
        self.0
    }
}

impl<N, E, M> FromRuntimeContext<N> for RunId
where
    E: Send + Clone + 'static,
    M: Send + Clone + 'static,
    N: GraphNode<Context = AgentContext<E, M>>,
{
    fn from_rtx(rtx: &RuntimeContext<N>) -> Result<Self, GraphError> {
        Ok(RunId(rtx.ctx.run_id))
    }
}

/// An extractor for the current session's ID from the agent state.
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn as_inner(&self) -> &Uuid {
        &self.0
    }

    pub fn into_inner(self) -> Uuid {
        self.0
    }
}

impl<N, E, M> FromRuntimeContext<N> for SessionId
where
    E: Send + Clone + 'static,
    M: Send + Clone + 'static,
    N: GraphNode<Context = AgentContext<E, M>>,
{
    fn from_rtx(rtx: &RuntimeContext<N>) -> Result<Self, GraphError> {
        Ok(SessionId(rtx.ctx.session_id))
    }
}

/// An extractor for the agent's identity information from the agent context.
pub struct Identity(pub AgentIdentity);

impl Identity {
    pub fn as_inner(&self) -> &AgentIdentity {
        &self.0
    }

    pub fn into_inner(self) -> AgentIdentity {
        self.0
    }
}

impl<N, E, M> FromRuntimeContext<N> for Identity
where
    E: Send + Clone + 'static,
    M: Send + Clone + 'static,
    N: GraphNode<Context = AgentContext<E, M>>,
{
    fn from_rtx(rtx: &RuntimeContext<N>) -> Result<Self, GraphError> {
        Ok(Identity(rtx.ctx.identity.clone()))
    }
}

/// An extractor for the prompt rendering environment from the agent context.
pub struct PromptEnvironment(pub PromptEnv);

impl PromptEnvironment {
    pub fn as_inner(&self) -> &PromptEnv {
        &self.0
    }

    pub fn into_inner(self) -> PromptEnv {
        self.0
    }
}

impl<N, E, M> FromRuntimeContext<N> for PromptEnvironment
where
    E: Send + Clone + 'static,
    M: Send + Clone + 'static,
    N: GraphNode<Context = AgentContext<E, M>>,
{
    fn from_rtx(rtx: &RuntimeContext<N>) -> Result<Self, GraphError> {
        Ok(PromptEnvironment(rtx.ctx.prompt_env.clone()))
    }
}

/// An extractor for the agent's name from the agent context.
pub struct Name(pub String);

impl Name {
    pub fn as_inner(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<N, E, M> FromRuntimeContext<N> for Name
where
    E: Send + Clone + 'static,
    M: Send + Clone + 'static,
    N: GraphNode<Context = AgentContext<E, M>>,
{
    fn from_rtx(rtx: &RuntimeContext<N>) -> Result<Self, GraphError> {
        Ok(Name(rtx.ctx.identity.name.clone()))
    }
}
