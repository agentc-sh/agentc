// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use json_patch::{Patch, patch};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, from_value, to_value};
use std::{
    any::Any,
    fmt::{Debug, Display},
    hash::Hash,
};

use crate::graph::errors::GraphError;

pub trait GraphState: Serialize + DeserializeOwned + Debug + Clone + Send + Sync {
    type Update: GraphStateUpdate<State = Self>;
    type Input: GraphStateInput<State = Self>;
}

pub trait GraphStateUpdate: Serialize + DeserializeOwned + Debug + Clone + Send + Sync {
    type State: GraphState<Update = Self>;

    fn apply(self, update: &mut Self::State);
    fn merge(self, other: Self) -> Self;

    fn try_merge_with(self, other: Option<Self>) -> Self {
        match other {
            Some(other) => self.merge(other),
            None => self,
        }
    }
}

pub trait IntoStateUpdate<U>
where
    U: GraphStateUpdate + Send,
{
    fn into_update(self) -> Result<Option<U>, GraphError>;
}

impl<U> IntoStateUpdate<U> for U
where
    U: GraphStateUpdate + Send,
{
    fn into_update(self) -> Result<Option<U>, GraphError> {
        Ok(Some(self))
    }
}

impl<U> IntoStateUpdate<U> for ()
where
    U: GraphStateUpdate + Send,
{
    fn into_update(self) -> Result<Option<U>, GraphError> {
        Ok(None)
    }
}

impl<U> IntoStateUpdate<U> for Patch
where
    U: GraphStateUpdate + Send + Default,
{
    fn into_update(self) -> Result<Option<U>, GraphError> {
        let mut update = to_value(U::default()).map_err(GraphError::conversion_error)?;
        patch(&mut update, &self).map_err(GraphError::conversion_error)?;
        from_value(update)
            .map_err(GraphError::conversion_error)
            .map(Some)
    }
}

pub trait FromStateUpdate<U>: Sized
where
    U: GraphStateUpdate + Send,
{
    fn from_update(update: U) -> Result<Option<Self>, GraphError>;
}

impl<T, U> FromStateUpdate<U> for T
where
    T: GraphState<Update = U> + Default,
    U: GraphStateUpdate<State = T> + Send,
{
    fn from_update(update: U) -> Result<Option<Self>, GraphError> {
        let mut state = Self::default();
        update.apply(&mut state);
        Ok(Some(state))
    }
}

pub trait AnyStateUpdate: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

impl<T: GraphStateUpdate + 'static> AnyStateUpdate for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl dyn AnyStateUpdate {
    pub fn downcast_ref<U: GraphStateUpdate + 'static>(&self) -> Option<&U> {
        self.as_any().downcast_ref::<U>()
    }

    pub fn downcast_mut<U: GraphStateUpdate + 'static>(&mut self) -> Option<&mut U> {
        self.as_any_mut().downcast_mut::<U>()
    }

    pub fn downcast<U: GraphStateUpdate + 'static>(
        self: Box<Self>,
    ) -> Result<U, Box<dyn AnyStateUpdate>> {
        if self.as_any().is::<U>() {
            Ok(*self.into_any().downcast::<U>().unwrap())
        } else {
            Err(self)
        }
    }
}

pub trait AnyState: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_value(&self) -> Value;
}

impl<S: GraphState + 'static> AnyState for S {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_value(&self) -> Value {
        to_value(self).unwrap_or(Value::Null)
    }
}

impl dyn AnyState {
    pub fn downcast_ref<S: GraphState + 'static>(&self) -> Option<&S> {
        self.as_any().downcast_ref::<S>()
    }
}

pub trait FromState<S: GraphState>: Sized {
    fn from_state(state: &S) -> Result<Option<Self>, GraphError>;
}

impl<S: GraphState> FromState<S> for () {
    fn from_state(_: &S) -> Result<Option<Self>, GraphError> {
        Ok(Some(()))
    }
}

impl<S: GraphState> FromState<S> for S {
    fn from_state(state: &S) -> Result<Option<Self>, GraphError> {
        Ok(Some(state.clone()))
    }
}

impl<S: GraphState> FromState<S> for Value {
    fn from_state(state: &S) -> Result<Option<Self>, GraphError> {
        to_value(state)
            .map(Some)
            .map_err(GraphError::conversion_error)
    }
}

pub trait GraphStateInput: Serialize + DeserializeOwned + Debug + Send + Sync {
    type State: GraphState<Input = Self>;

    fn initialize(self) -> Self::State;
}

pub trait GraphContext: Clone + Send + Sync {}

pub trait GraphNode: Display + Eq + Hash + Debug + Clone + Send + Sync {
    type Context: GraphContext;
    type State: GraphState;
}

pub type CtxOf<N> = <N as GraphNode>::Context;
pub type StateOf<N> = <N as GraphNode>::State;
pub type UpdateOf<N> = <StateOf<N> as GraphState>::Update;
pub type InputOf<N> = <StateOf<N> as GraphState>::Input;
