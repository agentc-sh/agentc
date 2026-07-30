// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::Utc;
use futures::stream::TryStreamExt;
use serde_json::{Value, to_string};
use std::{
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    hash::Hash,
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use tokio::sync::mpsc;
use uuid::Uuid;

use agentc_agent::{
    context::{AgentContext, Identity},
    graph::{
        command::GraphNodeCommand,
        context::{Ctx, State},
        errors::GraphError,
        runtime::{Graph, GraphBuilder},
        state::{GraphNode, GraphStateUpdate},
    },
    tools::{
        activity::{ActivityDelta, ActivityEmitter},
        dispatcher::DispatchOutcome,
        types::ToolExecutionContext,
    },
    types::{
        conversion::{FromModelType, ToModelType},
        identity::AgentIdentity,
        tools::{ToolCall, ToolDefinition},
    },
};
use agentc_model::{
    instrument::AsInstrumentedModel,
    middleware::{
        retry::{Retry, RetryPolicy},
        timeout::Timeout,
    },
    traits::{CompletionModel, CompletionModelExt},
    types::{reasoning::ReasoningContent, stream::CompletionStreamEvent},
};
use agentc_prompt::{
    buffer::{MessageBuffer, TokenBudget},
    macros::context,
    template::Role,
};
use agentc_telemetry::{Level, debug, error, info, instrument, warn};

use crate::{
    graph::{
        config::ReActGraphConfig,
        extractors::{ContextVars, Messages, Model, ModelConfig, ToolDefinitions, Tools},
        state::{ReActState, ReActStateUpdate},
    },
    types::{
        context_var::ContextVar,
        event::{Event, ReasoningSignatureSubtype},
        message::{AssistantMessage, Message, MessageList, ReasoningMessage, ToolMessage},
        model::ModelConfig as ReActModelConfig,
    },
};

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub enum ReActNode {
    Entrypoint,
    RouteNext,
    CallModel,
    CallTools,
}

impl Display for ReActNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            ReActNode::Entrypoint => write!(f, "entrypoint"),
            ReActNode::RouteNext => write!(f, "route_next"),
            ReActNode::CallModel => write!(f, "call_model"),
            ReActNode::CallTools => write!(f, "call_tools"),
        }
    }
}

impl FromStr for ReActNode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "entrypoint" => Ok(Self::Entrypoint),
            "route_next" => Ok(Self::RouteNext),
            "call_model" => Ok(Self::CallModel),
            "call_tools" => Ok(Self::CallTools),
            other => Err(anyhow::anyhow!("Invalid ReActNode: {}", other)),
        }
    }
}

impl GraphNode for ReActNode {
    type Context = AgentContext<Event, Message>;
    type State = ReActState;
}

impl ReActNode {
    pub fn graph(config: ReActGraphConfig) -> GraphBuilder<Self> {
        Graph::builder(Self::Entrypoint)
            .with_name("react")
            .with_node_fn(Self::Entrypoint, Self::entrypoint)
            .with_node_fn(Self::RouteNext, Self::route_next)
            .with_node_fn(
                Self::CallModel,
                move |Ctx(ctx): Ctx<AgentContext<Event, Message>>,
                      State(state): State<ReActState>,
                      Model(model): Model,
                      ModelConfig(model_config): ModelConfig,
                      Messages(messages): Messages,
                      ContextVars(context_vars): ContextVars,
                      ToolDefinitions(tool_definitions): ToolDefinitions,
                      Identity(identity): Identity| {
                    let config = config.clone();

                    async move {
                        Self::call_model(
                            ctx,
                            state,
                            model,
                            model_config,
                            config,
                            messages,
                            context_vars,
                            tool_definitions,
                            identity,
                        )
                        .await
                    }
                },
            )
            .with_node_fn(Self::CallTools, Self::call_tools)
    }

    #[instrument(
        level = Level::INFO,
        skip(ctx, state),
        fields(
            tenant_id = &ctx.tenant_id,
            session_id = ?ctx.session_id,
            run_id = ?ctx.run_id,
        )
    )]
    pub async fn entrypoint(
        Ctx(ctx): Ctx<AgentContext<Event, Message>>,
        State(state): State<ReActState>,
    ) -> Result<GraphNodeCommand<ReActNode>, GraphError> {
        debug!(
            event = "EnteredGraph",
            tenant_id = &ctx.tenant_id,
            session_id = ?ctx.session_id,
            run_id = ?ctx.run_id,
        );

        ctx.emit(Event::state_snapshot(state))
            .map_err(GraphError::execution_error)?;

        Ok(GraphNodeCommand::goto(ReActNode::RouteNext))
    }

    #[instrument(
        level = Level::INFO,
        skip(ctx, messages),
        fields(
            tenant_id = &ctx.tenant_id,
            session_id = ?ctx.session_id,
            run_id = ?ctx.run_id,
        )
    )]
    pub async fn route_next(
        Ctx(ctx): Ctx<AgentContext<Event, Message>>,
        Messages(messages): Messages,
    ) -> Result<GraphNodeCommand<ReActNode>, GraphError> {
        debug!(
            event = "RoutingNextStep",
            tenant_id = &ctx.tenant_id,
            session_id = ?ctx.session_id,
            run_id = ?ctx.run_id,
        );

        // If the last message is not from the model, we need to call the model
        if messages
            .last()
            .is_none_or(|m| m.as_assistant().is_none())
        {
            debug!(
                event = "RoutingToCallModel",
                tenant_id = &ctx.tenant_id,
                session_id = ?ctx.session_id,
                run_id = ?ctx.run_id,
            );

            return Ok(GraphNodeCommand::goto(ReActNode::CallModel));
        }

        // If the last message is an assistant message with tool calls, we need to
        // call the tools.
        if messages
            .last()
            .and_then(|m| m.as_assistant())
            .is_some_and(|a| a.has_tool_calls())
        {
            debug!(
                event = "RoutingToCallTools",
                tenant_id = &ctx.tenant_id,
                session_id = ?ctx.session_id,
                run_id = ?ctx.run_id,
            );

            return Ok(GraphNodeCommand::goto(ReActNode::CallTools));
        }

        debug!(
            event = "RoutingToEnd",
            tenant_id = &ctx.tenant_id,
            session_id = ?ctx.session_id,
            run_id = ?ctx.run_id,
        );

        ctx.emit(Event::messages_snapshot(messages))
            .map_err(GraphError::execution_error)?;

        Ok(GraphNodeCommand::end())
    }

    #[instrument(
        level = Level::INFO,
        skip(
            ctx,
            state,
            model,
            model_config,
            config,
            messages,
            tool_definitions,
            identity,
        ),
        fields(
            tenant_id = &ctx.tenant_id,
            session_id = ?ctx.session_id,
            run_id = ?ctx.run_id,
            message_ids = ?messages.iter().map(|m| m.id()).collect::<Vec<_>>(),
        )
    )]
    #[allow(clippy::too_many_arguments)]
    pub async fn call_model(
        ctx: AgentContext<Event, Message>,
        state: ReActState,
        model: Arc<dyn CompletionModel>,
        model_config: Option<ReActModelConfig>,
        config: ReActGraphConfig,
        messages: Vec<Message>,
        context_vars: Vec<ContextVar>,
        tool_definitions: Vec<ToolDefinition>,
        identity: AgentIdentity,
    ) -> Result<GraphNodeCommand<ReActNode>, GraphError> {
        info!(
            event = "CallingModel",
            tenant_id = &ctx.tenant_id,
            session_id = ?ctx.session_id,
            run_id = ?ctx.run_id,
            message_ids = ?messages.iter().map(|m| m.id()).collect::<Vec<_>>(),
        );

        if !matches!(messages.last(), Some(Message::User(_)) | Some(Message::Tool(_))) {
            info!(
                event = "ModelCallSkipped",
                reason = "Last message is not from user or tool",
                tenant_id = &ctx.tenant_id,
                session_id = ?ctx.session_id,
                run_id = ?ctx.run_id,
            );

            return Ok(GraphNodeCommand::goto(ReActNode::RouteNext));
        }

        let model = model
            .layer_with(
                model_config
                    .as_ref()
                    .and_then(|config| config.timeout)
                    .or(config.default_model_config.timeout),
                |timeout| Timeout::new(Duration::from_millis(timeout)),
            )
            .layer_with(
                model_config
                    .as_ref()
                    .and_then(|config| config.retry.clone())
                    .or(config.default_model_config.retry),
                |retry| {
                    Retry::new(RetryPolicy {
                        max_attempts: retry.max_attempts,
                        initial_backoff: Duration::from_millis(retry.initial_backoff),
                        max_backoff: Duration::from_millis(retry.max_backoff),
                    })
                },
            )
            .as_instrumented();

        let mut prompt_ctx = context!(
            tenant_id = &ctx.tenant_id,
            session_id = &ctx.session_id,
            run_id = &ctx.run_id,
            agent_name = &identity.name,
            context_vars = context_vars,
            tools = tool_definitions,
            messages = messages,
            state = &state.context,
            current_datetime = Utc::now(),
        );

        for contributor in &ctx.template_vars {
            match contributor.template_vars().await {
                Ok(vars) => prompt_ctx.merge(vars),
                Err(e) => {
                    warn!(
                        event = "TemplateVarsError",
                        error = %e,
                        tenant_id = &ctx.tenant_id,
                        session_id = ?ctx.session_id,
                        run_id = ?ctx.run_id,
                    );
                }
            }
        }

        let rendered_prompt = ctx
            .prompt_source
            .load()
            .await
            .map_err(GraphError::execution_error)?
            .render(&ctx.prompt_env, &prompt_ctx, ctx.token_counter.as_ref())
            .map_err(GraphError::execution_error)?;

        let override_params = model_config
            .and_then(|config| config.r#override)
            .and_then(|config| config.inference_params);

        // Build a single buffer for the entire context window. Rendered prompt
        // messages are pushed as pinned so compaction never removes them.
        let mut buffer = MessageBuffer::<Message>::builder()
            .with_budget(TokenBudget::new(
                override_params
                    .as_ref()
                    .and_then(|p| p.max_tokens)
                    .or(model.inference_params().max_tokens)
                    .unwrap_or(4096) as usize,
                2048,
            ))
            .with_counter_arc(ctx.token_counter.clone())
            .build();

        for rendered in rendered_prompt.into_messages() {
            buffer.push_pinned(match rendered.role {
                Role::System => Message::system(rendered.content),
                Role::User => Message::user(rendered.content),
                Role::Assistant => Message::assistant(rendered.content),
            });
        }

        buffer.extend(messages);
        buffer
            .compact_with(ctx.compaction_strategy.as_ref())
            .await;

        let mut stream = model
            .request(
                buffer
                    .into_messages()
                    .collect::<MessageList>()
                    .to_model_type(),
            )
            .tools(
                tool_definitions
                    .into_iter()
                    .map(|td| td.to_model_type()),
            )
            .maybe_provider_params(override_params.and_then(|p| p.provider_params))
            .send()
            .await
            .map_err(GraphError::execution_error)?;

        let message_id = Uuid::new_v4();
        let reasoning_id = Uuid::new_v4();
        let mut text_started = false;
        let mut reasoning_started = false;
        let mut text_content = String::new();
        let mut reasoning_content = String::new();
        let mut reasoning_message = None;
        let mut tool_calls = Vec::new();
        let mut token_usage = None;

        while let Some(chunk) = stream
            .try_next()
            .await
            .map_err(GraphError::execution_error)?
        {
            match chunk {
                CompletionStreamEvent::TextDelta { delta } => {
                    if !text_started {
                        ctx.emit(Event::text_message_start(message_id))
                            .map_err(GraphError::execution_error)?;
                        text_started = true;
                    }

                    ctx.emit(Event::text_message_content(message_id, &delta))
                        .map_err(GraphError::execution_error)?;
                    text_content.push_str(&delta);
                }
                CompletionStreamEvent::ReasoningDelta { delta, .. } => {
                    if !reasoning_started {
                        ctx.emit(Event::reasoning_start(reasoning_id))
                            .map_err(GraphError::execution_error)?;
                        ctx.emit(Event::reasoning_message_start(reasoning_id))
                            .map_err(GraphError::execution_error)?;
                        reasoning_started = true;
                    }

                    ctx.emit(Event::reasoning_message_content(reasoning_id, &delta))
                        .map_err(GraphError::execution_error)?;
                    reasoning_content.push_str(&delta);
                }
                CompletionStreamEvent::Reasoning(reasoning) => {
                    // The assembled reasoning block carries the signature or
                    // encrypted/redacted blob needed for multi-turn continuity.
                    let mut signature = None;

                    for content in reasoning.content {
                        match content {
                            ReasoningContent::Text { signature: Some(sig), .. } => {
                                signature = Some(sig)
                            }
                            ReasoningContent::Encrypted(e) => signature = Some(e),
                            ReasoningContent::Redacted(r) => signature = Some(r),
                            _ => {}
                        }
                    }

                    ctx.emit(Event::reasoning_message_end(reasoning_id))
                        .map_err(GraphError::execution_error)?;

                    if let Some(ref sig) = signature {
                        ctx.emit(Event::reasoning_signature(
                            reasoning_id,
                            ReasoningSignatureSubtype::Message,
                            reasoning_id.to_string(),
                            sig.clone(),
                        ))
                        .map_err(GraphError::execution_error)?;
                    }

                    ctx.emit(Event::reasoning_end(reasoning_id))
                        .map_err(GraphError::execution_error)?;

                    let message = ReasoningMessage::new(reasoning_content.clone())
                        .with_id(reasoning_id)
                        .with_tenant_id(ctx.tenant_id.clone())
                        .with_session_id(ctx.session_id)
                        .with_run_id(ctx.run_id);

                    reasoning_message = Some(match signature {
                        Some(sig) => message.with_signature(sig),
                        None => message,
                    });
                }
                CompletionStreamEvent::ToolCall(tool_call) => tool_calls.push(tool_call),
                CompletionStreamEvent::Done(usage) => token_usage = Some(usage),
                _ => {}
            }
        }

        if text_started {
            ctx.emit(Event::text_message_end(message_id))
                .map_err(GraphError::execution_error)?;
        }

        // If reasoning started but no assembled Reasoning block arrived (provider
        // only streams deltas without a final block), close out the events and
        // build the message from the accumulated delta content.
        if reasoning_started && reasoning_message.is_none() {
            ctx.emit(Event::reasoning_message_end(reasoning_id))
                .map_err(GraphError::execution_error)?;
            ctx.emit(Event::reasoning_end(reasoning_id))
                .map_err(GraphError::execution_error)?;

            reasoning_message = Some(
                ReasoningMessage::new(reasoning_content)
                    .with_id(reasoning_id)
                    .with_tenant_id(ctx.tenant_id.clone())
                    .with_session_id(ctx.session_id)
                    .with_run_id(ctx.run_id),
            );
        }

        info!(
            event = "ModelCallCompleted",
            tenant_id = &ctx.tenant_id,
            session_id = ?ctx.session_id,
            run_id = ?ctx.run_id,
            message_id = ?message_id,
            text_length = text_content.chars().count(),
            tool_calls = ?tool_calls.iter().map(|tc| tc.id.clone()).collect::<Vec<_>>(),
            input_tokens = &token_usage.as_ref().map(|u| u.input_tokens),
            output_tokens = &token_usage.as_ref().map(|u| u.output_tokens),
            cached_input_tokens = &token_usage.as_ref().and_then(|u| u.cache_input_tokens),
        );

        let mut messages = Vec::new();

        if let Some(reasoning) = reasoning_message {
            messages.push(Message::Reasoning(reasoning));
        }

        messages.push(Message::Assistant(
            AssistantMessage::new()
                .with_id(message_id)
                .with_tenant_id(ctx.tenant_id.clone())
                .with_session_id(ctx.session_id)
                .with_run_id(ctx.run_id)
                .maybe_with_content(if text_content.is_empty() {
                    None
                } else {
                    Some(text_content)
                })
                .with_name(identity.name)
                .with_tool_calls(
                    tool_calls
                        .into_iter()
                        .map(ToolCall::from_model_type)
                        .collect(),
                ),
        ));

        Ok(GraphNodeCommand::goto_and_update(
            ReActNode::RouteNext,
            ReActStateUpdate::new().with_messages(messages),
        ))
    }

    #[instrument(
        level = Level::INFO,
        skip(ctx, messages, tools),
        fields(
            tenant_id = &ctx.tenant_id,
            session_id = ?ctx.session_id,
            run_id = ?ctx.run_id,
            message_ids = ?messages.iter().map(|m| m.id()).collect::<Vec<_>>(),
        )
    )]
    pub async fn call_tools(
        Ctx(ctx): Ctx<AgentContext<Event, Message>>,
        State(state): State<ReActState>,
        Messages(messages): Messages,
        Tools(tools): Tools,
    ) -> Result<GraphNodeCommand<ReActNode>, GraphError> {
        info!(
            event = "CallingTools",
            tenant_id = &ctx.tenant_id,
            session_id = ?ctx.session_id,
            run_id = ?ctx.run_id,
            message_ids = ?messages.iter().map(|m| m.id()).collect::<Vec<_>>(),
        );

        let Some(assistant_message) = messages
            .last()
            .and_then(|m| m.as_assistant())
        else {
            info!(
                event = "ToolCallSkipped",
                reason = "Last message is not from assistant",
                tenant_id = &ctx.tenant_id,
                session_id = ?ctx.session_id,
                run_id = ?ctx.run_id,
            );

            return Ok(GraphNodeCommand::goto(ReActNode::RouteNext));
        };
        let Some(tool_calls) = &assistant_message.tool_calls else {
            info!(
                event = "ToolCallSkipped",
                reason = "No tool calls in last assistant message",
                tenant_id = &ctx.tenant_id,
                session_id = ?ctx.session_id,
                run_id = ?ctx.run_id,
            );

            return Ok(GraphNodeCommand::goto(ReActNode::RouteNext));
        };

        let mut results = Vec::new();
        let mut update = None;

        for tool_call in tool_calls {
            let message_id = Uuid::new_v4();

            ctx.emit(Event::tool_call_start(tool_call.id.clone(), tool_call.name.clone()))
                .map_err(GraphError::execution_error)?;

            ctx.emit(Event::tool_call_args(
                tool_call.id.clone(),
                to_string(&tool_call.arguments).map_err(GraphError::execution_error)?,
            ))
            .map_err(GraphError::execution_error)?;

            let (activity_tx, mut activity_rx) = mpsc::channel::<ActivityDelta>(32);
            let drain_emitter = ctx.emitter.clone();
            let drain_call_id = tool_call.id.clone();

            let drain_handle = tokio::spawn(async move {
                while let Some(delta) = activity_rx.recv().await {
                    let _ = drain_emitter.emit(Event::activity_delta(
                        &drain_call_id,
                        delta.activity_type,
                        delta.patch,
                    ));
                }
            });

            match tools
                .dispatch::<ReActState>(
                    tool_call.clone(),
                    &state,
                    ToolExecutionContext {
                        tenant_id: ctx.tenant_id.clone(),
                        session_id: ctx.session_id,
                        run_id: ctx.run_id,
                    },
                    Some(ActivityEmitter::new(activity_tx)),
                )
                .await
            {
                DispatchOutcome::Success { call_id, content, state_update } => {
                    drain_handle
                        .await
                        .map_err(GraphError::execution_error)?;

                    if let Some(state_update) = &state_update {
                        ctx.emit(Event::state_delta(state_update.clone()))
                            .map_err(GraphError::execution_error)?;
                    }

                    info!(
                        event = "ToolCallSuccess",
                        tenant_id = &ctx.tenant_id,
                        session_id = ?ctx.session_id,
                        run_id = ?ctx.run_id,
                        tool_call_id = &call_id,
                        tool_name = &tool_call.name,
                        content = ?content,
                        has_update = state_update.is_some(),
                    );

                    update = Some(
                        update
                            .take()
                            .unwrap_or_else(ReActStateUpdate::new)
                            .try_merge_with(state_update),
                    );

                    ctx.emit(Event::tool_call_result(&call_id, message_id, content.clone()))
                        .map_err(GraphError::execution_error)?;

                    results.push(Message::Tool(
                        ToolMessage::new(call_id)
                            .with_id(message_id)
                            .with_name(tool_call.name.clone())
                            .with_parent_message_id(*assistant_message.id())
                            .with_content(match content {
                                Value::String(s) => s,
                                other => to_string(&other).map_err(GraphError::execution_error)?,
                            }),
                    ));
                }
                DispatchOutcome::AwaitingClient(_) => {
                    drain_handle
                        .await
                        .map_err(GraphError::execution_error)?;

                    info!(
                        event = "ToolCallAwaitingClient",
                        tenant_id = &ctx.tenant_id,
                        session_id = ?ctx.session_id,
                        run_id = ?ctx.run_id,
                        tool_call_id = &tool_call.id,
                        tool_name = &tool_call.name,
                    );
                }
                DispatchOutcome::NotFound { call_id, name } => {
                    drain_handle
                        .await
                        .map_err(GraphError::execution_error)?;

                    let error = format!("Tool '{}' not found", name);

                    error!(
                        event = "ToolCallError",
                        reason = "Tool not found",
                        tenant_id = &ctx.tenant_id,
                        session_id = ?ctx.session_id,
                        run_id = ?ctx.run_id,
                        tool_call_id = &call_id,
                        tool_name = &name,
                    );

                    ctx.emit(Event::tool_call_error(&call_id, message_id, &error, None::<String>))
                        .map_err(GraphError::execution_error)?;

                    results.push(Message::Tool(
                        ToolMessage::new(call_id)
                            .with_id(message_id)
                            .with_name(tool_call.name.clone())
                            .with_parent_message_id(*assistant_message.id())
                            .with_content(error),
                    ));
                }
                DispatchOutcome::Error { call_id, error } => {
                    drain_handle
                        .await
                        .map_err(GraphError::execution_error)?;

                    error!(
                        event = "ToolCallError",
                        reason = "Execution error",
                        tenant_id = &ctx.tenant_id,
                        session_id = ?ctx.session_id,
                        run_id = ?ctx.run_id,
                        tool_call_id = &call_id,
                        tool_name = &tool_call.name,
                        error = &error,
                    );

                    ctx.emit(Event::tool_call_error(
                        &call_id,
                        assistant_message.id,
                        &error,
                        None::<String>,
                    ))
                    .map_err(GraphError::execution_error)?;

                    results.push(Message::Tool(
                        ToolMessage::new(call_id)
                            .with_id(message_id)
                            .with_name(tool_call.name.clone())
                            .with_parent_message_id(*assistant_message.id())
                            .with_content(error.to_string()),
                    ));
                }
            }

            ctx.emit(Event::tool_call_end(&tool_call.id))
                .map_err(GraphError::execution_error)?;
        }

        if results.is_empty() {
            info!(
                event = "NoToolResults",
                tenant_id = &ctx.tenant_id,
                session_id = ?ctx.session_id,
                run_id = ?ctx.run_id,
            );

            return Ok(GraphNodeCommand::goto(ReActNode::RouteNext));
        }

        Ok(GraphNodeCommand::goto_and_update(
            ReActNode::RouteNext,
            ReActStateUpdate::new()
                .with_messages(
                    results
                        .into_iter()
                        .map(|m| m.with_context(&ctx)),
                )
                .try_merge_with(update),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use futures::stream;
    use std::sync::atomic::{AtomicU32, Ordering};
    use tokio::time::sleep;

    use agentc_agent::{
        graph::state::GraphStateInput,
        stream::EventEmitter,
        tools::registry::ToolRegistry,
        types::capability::{CapabilityPolicy, CapabilitySet},
    };
    use agentc_model::{
        errors::ModelError,
        registry::ModelRegistry,
        stream::ChatCompletionStream,
        types::{
            identity::{ModelId, ProviderId},
            inference::InferenceParams,
            request::CompletionRequest,
            stream::CompletionStreamEvent,
        },
    };
    use agentc_prompt::{
        compaction::NoCompaction,
        counter::CharApproxCounter,
        env::{PromptContext, PromptEnv},
        source::{ConstantPromptSource, PromptSource},
        template::PromptTemplate,
    };
    use serde_json::json;

    use crate::{graph::state::ReActStateInput, types::model::ModelConfigRetry};

    struct StubModel {
        model_id: ModelId,
        params: InferenceParams,
        calls: AtomicU32,
        delay: Duration,
        fail: bool,
    }

    impl StubModel {
        fn new(delay: Duration, fail: bool) -> Self {
            Self {
                model_id: "test-model".into(),
                params: InferenceParams::default(),
                calls: AtomicU32::new(0),
                delay,
                fail,
            }
        }
    }

    #[async_trait]
    impl CompletionModel for StubModel {
        fn provider(&self) -> ProviderId {
            "stub".into()
        }

        fn otel_provider_name(&self) -> &'static str {
            "stub"
        }

        fn model(&self) -> &ModelId {
            &self.model_id
        }

        fn inference_params(&self) -> &InferenceParams {
            &self.params
        }

        async fn send(
            &self,
            _request: CompletionRequest,
        ) -> Result<ChatCompletionStream, ModelError> {
            self.calls
                .fetch_add(1, Ordering::SeqCst);
            sleep(self.delay).await;

            if self.fail {
                return Err(ModelError::transient(
                    "stub",
                    "temporary",
                    None,
                    None::<std::io::Error>,
                ));
            }

            Ok(ChatCompletionStream::new(stream::empty::<
                Result<CompletionStreamEvent, ModelError>,
            >()))
        }
    }

    struct CallModelHarness;

    impl CallModelHarness {
        fn identity() -> AgentIdentity {
            AgentIdentity {
                name: "test-agent".to_string(),
                provider: "stub".to_string(),
                model: "test-model".to_string(),
                capabilities: CapabilitySet::default(),
                capability_policy: CapabilityPolicy::default(),
            }
        }

        fn context(identity: AgentIdentity) -> AgentContext<Event, Message> {
            AgentContext {
                emitter: EventEmitter::new_pair().0,
                model_registry: ModelRegistry::new(),
                tool_registry: ToolRegistry::empty(),
                identity,
                prompt_env: PromptEnv::default(),
                prompt_source: Arc::new(ConstantPromptSource::new(PromptTemplate::system("test"))),
                token_counter: Arc::new(CharApproxCounter),
                compaction_strategy: Arc::new(NoCompaction),
                session_id: Uuid::new_v4(),
                run_id: Uuid::new_v4(),
                tenant_id: "test-tenant".to_string(),
                template_vars: Vec::new(),
            }
        }

        async fn call(
            model: Arc<StubModel>,
            model_config: Option<ReActModelConfig>,
            config: ReActGraphConfig,
        ) -> Result<GraphNodeCommand<ReActNode>, GraphError> {
            let identity = Self::identity();

            ReActNode::call_model(
                Self::context(identity.clone()),
                ReActStateInput::default().initialize(),
                model,
                model_config,
                config,
                vec![Message::user("hello")],
                Vec::new(),
                Vec::new(),
                identity,
            )
            .await
        }
    }

    #[tokio::test]
    async fn client_model_policy_precedes_graph_defaults() {
        let model = Arc::new(StubModel::new(Duration::from_millis(20), true));

        let result = CallModelHarness::call(
            model.clone(),
            Some(
                ReActModelConfig::new()
                    .with_timeout(100)
                    .with_retry(ModelConfigRetry {
                        max_attempts: 2,
                        initial_backoff: 0,
                        max_backoff: 0,
                    }),
            ),
            ReActGraphConfig {
                default_model_config: ReActModelConfig::new()
                    .with_timeout(1)
                    .with_retry(ModelConfigRetry {
                        max_attempts: 3,
                        initial_backoff: 0,
                        max_backoff: 0,
                    }),
            },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(model.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn graph_model_policy_fills_missing_client_values() {
        let model = Arc::new(StubModel::new(Duration::from_millis(20), false));

        let result = CallModelHarness::call(
            model.clone(),
            Some(ReActModelConfig::new()),
            ReActGraphConfig {
                default_model_config: ReActModelConfig::new()
                    .with_timeout(1)
                    .with_retry(ModelConfigRetry {
                        max_attempts: 2,
                        initial_backoff: 0,
                        max_backoff: 0,
                    }),
            },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(model.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn renders_prompt_from_source() {
        let source =
            ConstantPromptSource::new(PromptTemplate::system("from-source {{ agent_name }}"));

        let rendered = source
            .load()
            .await
            .unwrap()
            .render(
                &PromptEnv::default(),
                &PromptContext::from_json(json!({ "agent_name": "test-agent" })),
                &CharApproxCounter,
            )
            .unwrap();

        assert_eq!(rendered.messages()[0].content, "from-source test-agent");
    }
}
