// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_value};
use std::{marker::PhantomData, sync::Arc};

use crate::{
    graph::state::{AnyState, FromState, GraphState, IntoStateUpdate},
    tools::{
        errors::ToolError,
        types::{ToolInput, ToolOutput, ToolResponse, TypedToolInput, TypedToolOutput},
    },
    types::{capability::CapabilitySet, tools::ToolDefinition},
};

/// A trait representing a tool that can be executed by the agent.
#[async_trait]
pub trait Tool<S>: Send + Sync
where
    S: GraphState + 'static,
{
    type State: FromState<S> + Send;
    type StateUpdate: IntoStateUpdate<S::Update> + Send;

    /// Returns the definition of the tool, including its name, description, and parameter schema.
    fn definition(&self) -> ToolDefinition;

    /// Returns the capabilities required to execute this tool.
    fn capabilities(&self) -> CapabilitySet;

    /// Executes the tool with the given arguments, returning either a successful output or an error.
    async fn execute(
        &self,
        input: ToolInput<Self::State>,
    ) -> Result<ToolOutput<Self::StateUpdate>, ToolError>;
}

/// A trait for tools with strongly typed input and output.
#[async_trait]
pub trait TypedTool<S>: Send + Sync
where
    S: GraphState + 'static,
{
    type Input: for<'de> Deserialize<'de> + JsonSchema + Send;
    type Output: Serialize + Send;
    type State: FromState<S> + Send;
    type StateUpdate: IntoStateUpdate<S::Update> + Send;

    /// Returns the name of the tool.
    fn name(&self) -> &str;

    /// Returns a description of the tool.
    fn description(&self) -> &str;

    /// Returns the capabilities required to execute this tool.
    fn capabilities(&self) -> CapabilitySet;

    /// Executes the tool with the given typed input, returning either a typed output or an error.
    async fn execute(
        &self,
        input: TypedToolInput<Self::Input, Self::State>,
    ) -> Result<TypedToolOutput<Self::Output, Self::StateUpdate>, ToolError>;
}

pub struct TypedToolWrapper<T, S> {
    tool: T,
    _marker: PhantomData<S>,
}

impl<T, S> TypedToolWrapper<T, S> {
    pub fn new(tool: T) -> Self {
        Self { tool, _marker: PhantomData }
    }
}

#[async_trait]
impl<S, T> Tool<S> for TypedToolWrapper<T, S>
where
    S: GraphState + 'static,
    T: TypedTool<S> + Send + Sync,
{
    type State = T::State;
    type StateUpdate = T::StateUpdate;

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.tool.name().to_string(),
            description: self.tool.description().to_string(),
            parameters: serde_json::to_value(schema_for!(T::Input)).unwrap_or(Value::Null),
        }
    }

    fn capabilities(&self) -> CapabilitySet {
        self.tool.capabilities()
    }

    async fn execute(
        &self,
        input: ToolInput<Self::State>,
    ) -> Result<ToolOutput<Self::StateUpdate>, ToolError> {
        let typed_args = match from_value(input.args) {
            Ok(i) => i,
            Err(e) => return Err(ToolError::InvalidArguments(e.to_string())),
        };

        match self
            .tool
            .execute(
                TypedToolInput::new(typed_args, input.context)
                    .maybe_with_activity_emitter(input.emitter)
                    .maybe_with_state(input.state),
            )
            .await
        {
            Ok(response) => Ok(ToolOutput {
                output: serde_json::to_value(response.output).unwrap_or(Value::Null),
                state_update: response.state_update,
            }),
            Err(e) => Err(e),
        }
    }
}

/// A type erased tool.
#[async_trait]
pub trait ErasedTool: Send + Sync {
    /// Returns the definition of the tool, including its name, description, and parameter schema.
    fn definition(&self) -> ToolDefinition;

    /// Returns the capabilities required to execute this tool.
    fn capabilities(&self) -> CapabilitySet;

    /// Executes the tool with the given arguments, returning either a successful output or an error.
    async fn execute(&self, input: ToolInput<Arc<dyn AnyState>>) -> ToolResponse;
}

/// A wrapper that erases the type of a tool.
pub struct ErasedToolWrapper<T, S> {
    tool: T,
    _marker: PhantomData<S>,
}

impl<T, S> ErasedToolWrapper<T, S> {
    pub fn new(tool: T) -> Self {
        Self { tool, _marker: PhantomData }
    }
}

#[async_trait]
impl<T, S> ErasedTool for ErasedToolWrapper<T, S>
where
    S: GraphState + 'static,
    T: Tool<S> + 'static,
    T::StateUpdate: IntoStateUpdate<S::Update> + Send,
{
    fn definition(&self) -> ToolDefinition {
        self.tool.definition()
    }

    fn capabilities(&self) -> CapabilitySet {
        self.tool.capabilities()
    }

    async fn execute(&self, input: ToolInput<Arc<dyn AnyState>>) -> ToolResponse {
        let typed_state = match input
            .state
            .as_ref()
            .and_then(|any| any.downcast_ref::<S>())
            .map(|s| T::State::from_state(s))
            .transpose()
        {
            Ok(opt) => opt.flatten(),
            Err(err) => return ToolResponse::err(err),
        };

        match self
            .tool
            .execute(ToolInput {
                args: input.args,
                context: input.context,
                emitter: input.emitter,
                state: typed_state,
            })
            .await
        {
            Ok(output) => match output.state_update {
                Some(update) => match update.into_update() {
                    Ok(Some(update)) => ToolResponse::ok_with_state(output.output, update),
                    Ok(None) => ToolResponse::ok(output.output),
                    Err(err) => ToolResponse::err(err),
                },
                None => ToolResponse::ok(output.output),
            },
            Err(e) => ToolResponse::err(e),
        }
    }
}

type FnToolMarker<I, O, SU, S> = fn(I) -> (O, SU, S);

/// A helper struct to create tools from async functions.
pub struct FnTool<F, I, O, SU, S> {
    name: &'static str,
    description: &'static str,
    capabilities: CapabilitySet,
    f: F,
    _marker: PhantomData<FnToolMarker<I, O, SU, S>>,
}

impl<F, I, O, SU, S, Fut> FnTool<F, I, O, SU, S>
where
    F: Fn(TypedToolInput<I>) -> Fut + Send + Sync,
    I: for<'de> Deserialize<'de> + JsonSchema + Send,
    O: Serialize + Send,
    SU: IntoStateUpdate<S::Update> + Send + Sync + 'static,
    S: GraphState + 'static,
    Fut: Future<Output = Result<TypedToolOutput<O, SU>, ToolError>> + Send,
{
    pub fn new(
        name: &'static str,
        description: &'static str,
        capabilities: CapabilitySet,
        f: F,
    ) -> Self {
        Self {
            name,
            description,
            capabilities,
            f,
            _marker: PhantomData,
        }
    }
}

#[async_trait]
impl<F, I, O, SU, S, Fut> TypedTool<S> for FnTool<F, I, O, SU, S>
where
    F: Fn(TypedToolInput<I>) -> Fut + Send + Sync,
    I: for<'de> Deserialize<'de> + JsonSchema + Send,
    O: Serialize + Send,
    SU: IntoStateUpdate<S::Update> + Send + Sync + 'static,
    S: GraphState + 'static,
    Fut: Future<Output = Result<TypedToolOutput<O, SU>, ToolError>> + Send,
{
    type Input = I;
    type Output = O;
    type State = ();
    type StateUpdate = SU;

    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        self.description
    }
    fn capabilities(&self) -> CapabilitySet {
        self.capabilities.clone()
    }

    async fn execute(
        &self,
        input: TypedToolInput<Self::Input, Self::State>,
    ) -> Result<TypedToolOutput<O, SU>, ToolError> {
        (self.f)(
            TypedToolInput::new(input.args, input.context)
                .maybe_with_activity_emitter(input.emitter),
        )
        .await
    }
}
