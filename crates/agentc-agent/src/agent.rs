// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use anyhow::Result;
use std::sync::Arc;
use tokio::task::JoinHandle;

use agentc_model::registry::ModelRegistry;
use agentc_prompt::{
    compaction::{CompactionStrategy, NoCompaction},
    counter::{CharApproxCounter, TokenCounter},
    env::PromptEnv,
    vars::TemplateVars,
};

use crate::{
    context::AgentContext,
    errors::AgentError,
    graph::{
        checkpoint::types::RunStatus,
        runtime::{Graph, RunOutcome, SessionConfig},
        state::{GraphNode, InputOf, StateOf},
    },
    stream::{EventEmitter, EventStream},
    tools::{
        registry::ToolRegistry,
        traits::{Tool, TypedTool},
    },
    types::{event::AgentEvent, identity::AgentIdentity, params::RunParams},
};

/// The agent runtime harness, which manages the execution of the agent graph and the flow of data.
///
/// The `M` parameter is the message type used by the compaction strategy. It
/// defaults to `()`, which pairs with `NoCompaction` for graphs that do not
/// require context-window management.
pub struct Agent<N, E, M = ()>
where
    N: GraphNode<Context = AgentContext<E, M>>,
    E: From<AgentEvent<StateOf<N>>> + Send + Clone + 'static,
    M: Send + Clone + 'static,
{
    graph: Arc<Graph<N>>,
    identity: AgentIdentity,
    model_registry: ModelRegistry,
    tool_registry: ToolRegistry,
    prompt_env: PromptEnv,
    token_counter: Arc<dyn TokenCounter>,
    compaction_strategy: Arc<dyn CompactionStrategy<M>>,
    template_vars: Vec<Arc<dyn TemplateVars>>,
}

impl<N, E, M> Agent<N, E, M>
where
    N: GraphNode<Context = AgentContext<E, M>> + 'static,
    E: From<AgentEvent<StateOf<N>>> + Send + Clone + 'static,
    M: Send + Clone + 'static,
{
    /// Get a new builder for constructing an [`Agent`](crate::agent::Agent) with custom configuration.
    pub fn builder() -> AgentBuilder<N, E, M> {
        AgentBuilder::new()
    }

    /// Get a reference to the identity of this agent.
    pub fn identity(&self) -> &AgentIdentity {
        &self.identity
    }

    /// Run the agent with the given input, returning a stream of events that represent the execution.
    pub async fn run(
        &self,
        params: RunParams<InputOf<N>>,
    ) -> Result<(EventStream<E>, JoinHandle<()>), AgentError> {
        let (emitter, event_stream) = EventEmitter::<E>::new_pair();
        let graph = self.graph.clone();
        let session_id = params.session_id;
        let run_id = params.run_id;
        let checkpoint_id = params.checkpoint_id;
        let resume_payload = params.resume_payload.clone();
        let tenant_id = params.tenant_id.clone();
        let model_registry = self.model_registry.clone();
        let tool_registry = self.tool_registry.clone();
        let identity = self.identity.clone();
        let prompt_env = self.prompt_env.clone();
        let token_counter = self.token_counter.clone();
        let compaction_strategy = self.compaction_strategy.clone();
        let template_vars = self.template_vars.clone();

        let handle = tokio::spawn(async move {
            emitter
                .emit(AgentEvent::run_started(session_id, run_id).into())
                .ok();

            match graph
                .run(
                    AgentContext {
                        emitter: emitter.clone(),
                        model_registry,
                        tool_registry,
                        identity,
                        prompt_env,
                        token_counter,
                        compaction_strategy,
                        session_id,
                        run_id,
                        tenant_id: tenant_id.clone(),
                        template_vars,
                    },
                    params.input,
                    SessionConfig {
                        session_id,
                        run_id,
                        tenant_id,
                        checkpoint_id,
                        resume_payload,
                    },
                )
                .await
            {
                Ok(result) => {
                    emitter
                        .emit(
                            AgentEvent::run_finished(
                                session_id,
                                run_id,
                                match &result {
                                    RunOutcome::Completed(_) => RunStatus::Completed,
                                    RunOutcome::Interrupted { .. } => RunStatus::Interrupted,
                                },
                                match &result {
                                    RunOutcome::Completed(_) => None,
                                    RunOutcome::Interrupted { payload, .. } => payload.clone(),
                                },
                                Some(result.into_state()),
                            )
                            .into(),
                        )
                        .ok();
                }
                Err(e) => {
                    emitter
                        .emit(
                            AgentEvent::run_error(session_id, run_id, format!("{}", e), None)
                                .into(),
                        )
                        .ok();
                }
            }
        });

        Ok((event_stream, handle))
    }
}

pub struct AgentBuilder<N, E, M = ()>
where
    N: GraphNode<Context = AgentContext<E, M>> + 'static,
    E: From<AgentEvent<StateOf<N>>> + Send + Clone + 'static,
    M: Send + Clone + 'static,
{
    graph: Option<Graph<N>>,
    identity: Option<AgentIdentity>,
    model_registry: Option<ModelRegistry>,
    tool_registry: ToolRegistry,
    tool_registries: Vec<ToolRegistry>,
    prompt_env: PromptEnv,
    token_counter: Arc<dyn TokenCounter>,
    compaction_strategy: Arc<dyn CompactionStrategy<M>>,
    template_vars: Vec<Arc<dyn TemplateVars>>,
}

impl<N, E, M> Default for AgentBuilder<N, E, M>
where
    N: GraphNode<Context = AgentContext<E, M>> + 'static,
    E: From<AgentEvent<StateOf<N>>> + Send + Clone + 'static,
    M: Send + Clone + 'static,
NoCompaction: CompactionStrategy<M>,
 {
    fn default() -> Self {
        Self::new()
    }
}

impl<N, E, M> AgentBuilder<N, E, M>
where
    N: GraphNode<Context = AgentContext<E, M>> + 'static,
    E: From<AgentEvent<StateOf<N>>> + Send + Clone + 'static,
    M: Send + Clone + 'static,
{
    pub fn new() -> Self
    where
        NoCompaction: CompactionStrategy<M>,
    {
        AgentBuilder {
            graph: None,
            identity: None,
            model_registry: None,
            tool_registry: ToolRegistry::empty(),
            tool_registries: Vec::new(),
            prompt_env: PromptEnv::default(),
            token_counter: Arc::new(CharApproxCounter),
            compaction_strategy: Arc::new(NoCompaction),
            template_vars: Vec::new(),
        }
    }

    /// Set the graph that defines the agent's behavior. This is required.
    pub fn with_graph(mut self, graph: impl Into<Graph<N>>) -> Self {
        self.graph = Some(graph.into());
        self
    }

    /// Set the agent's identity, which is included in emitted events. This is required.
    pub fn with_identity(mut self, identity: impl Into<AgentIdentity>) -> Self {
        self.identity = Some(identity.into());
        self
    }

    /// Set the model registry used for looking up models in the graph. This is required.
    pub fn with_model_registry(mut self, model_registry: impl Into<ModelRegistry>) -> Self {
        self.model_registry = Some(model_registry.into());
        self
    }

    /// Set the Jinja2 rendering environment used for prompt template renders.
    /// Defaults to a strict-mode environment with no custom functions or filters.
    pub fn with_prompt_env(mut self, env: PromptEnv) -> Self {
        self.prompt_env = env;
        self
    }

    /// Set the token counter used for budget tracking and compaction.
    /// Defaults to `CharApproxCounter`. For production use, prefer
    /// [`TiktokenCounter::o200k_base()`](agentc_prompt::counter::TiktokenCounter::o200k_base()).
    pub fn with_token_counter(mut self, counter: impl TokenCounter + 'static) -> Self {
        self.token_counter = Arc::new(counter);
        self
    }

    /// Set the compaction strategy applied to the message buffer before each model call.
    /// Defaults to `NoCompaction`.
    pub fn with_compaction_strategy(
        mut self,
        strategy: impl CompactionStrategy<M> + 'static,
    ) -> Self {
        self.compaction_strategy = Arc::new(strategy);
        self
    }

    /// Add a tool registry to the agent. Tools from all registries will be merged into a single effective registry for execution.
    pub fn with_tool_registry(mut self, tool_registry: impl Into<ToolRegistry>) -> Self {
        self.tool_registries
            .push(tool_registry.into());
        self
    }

    /// Add a [`TemplateVars`](agentc_prompt::vars::TemplateVars) contributor to the agent.
    ///
    /// Contributors are called in `call_model` before each prompt render. Their
    /// returned variables are merged into the prompt context.
    pub fn with_template_vars(mut self, contributor: Arc<dyn TemplateVars>) -> Self {
        self.template_vars.push(contributor);
        self
    }

    /// Add multiple [`TemplateVars`](agentc_prompt::vars::TemplateVars) contributors at once.
    pub fn with_all_template_vars(
        mut self,
        contributors: impl IntoIterator<Item = Arc<dyn TemplateVars>>,
    ) -> Self {
        self.template_vars.extend(contributors);
        self
    }

    /// Add multiple tool registries to the agent.
    pub fn with_tool_registries(
        mut self,
        tool_registries: impl IntoIterator<Item = impl Into<ToolRegistry>>,
    ) -> Self {
        self.tool_registries.extend(
            tool_registries
                .into_iter()
                .map(Into::into),
        );
        self
    }

    /// Add a single tool to the agent's default tool registry.
    pub fn with_tool<T>(mut self, tool: T) -> Self
    where
        T: Tool<StateOf<N>> + 'static,
    {
        self.tool_registry.register(tool);
        self
    }

    /// Add a typed tool to the agent's default tool registry.
    pub fn with_typed_tool<T>(mut self, tool: T) -> Self
    where
        T: TypedTool<StateOf<N>> + 'static,
    {
        self.tool_registry.register_typed(tool);
        self
    }

    /// Build the [`Agent`](crate::agent::Agent) with the provided configuration. Returns an error if any required fields are missing.
    pub fn build(self) -> Result<Agent<N, E, M>, AgentError> {
        Ok(Agent {
            graph: Arc::new(self.graph.ok_or(AgentError::configuration("Graph is required"))?),
            identity: self.identity.ok_or(AgentError::configuration("Identity is required"))?,
            model_registry: self.model_registry.ok_or(AgentError::configuration("Model registry is required"))?,
            tool_registry: self.tool_registries.into_iter().fold(self.tool_registry, |acc, r| acc.merged_with(r)),
            prompt_env: self.prompt_env,
            token_counter: self.token_counter,
            compaction_strategy: self.compaction_strategy,
            template_vars: self.template_vars,
        })
    }
}
