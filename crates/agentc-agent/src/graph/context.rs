// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde_json::Value;

use crate::graph::{
    errors::GraphError,
    state::{CtxOf, GraphNode, StateOf},
};

pub struct RuntimeContext<N: GraphNode> {
    pub ctx: CtxOf<N>,
    pub state: StateOf<N>,
    pub resume_payload: Option<Value>,
}

pub trait FromRuntimeContext<N: GraphNode> {
    fn from_rtx(rtx: &RuntimeContext<N>) -> Result<Self, GraphError>
    where
        Self: Sized;
}

pub struct State<S>(pub S);
impl<N: GraphNode> FromRuntimeContext<N> for State<StateOf<N>> {
    fn from_rtx(rtx: &RuntimeContext<N>) -> Result<Self, GraphError> {
        Ok(State(rtx.state.clone()))
    }
}

pub struct Ctx<C>(pub C);
impl<N: GraphNode> FromRuntimeContext<N> for Ctx<N::Context>
where
    N::Context: Clone,
{
    fn from_rtx(rtx: &RuntimeContext<N>) -> Result<Self, GraphError> {
        Ok(Ctx(rtx.ctx.clone()))
    }
}

/// An extractor that provides the ability to interrupt a run and resume it later with a payload.
///
/// Declare this as a parameter in a node function alongside other extractors. On first run,
/// calling [`.interrupt(payload)?`](Interrupt::interrupt) raises
/// [`GraphError::Interrupt`](crate::graph::errors::GraphError::Interrupt), which propagates
/// up to [`Graph::run`](crate::graph::graph::Graph::run) and ends the run cleanly with
/// [`RunStatus::Interrupted`](crate::graph::checkpoint::types::RunStatus::Interrupted).
///
/// On resume, the same call returns `Ok(resume_value)` where `resume_value` is whatever
/// the caller supplied in
/// [`SessionConfig::resume_payload`](crate::graph::graph::SessionConfig::resume_payload),
/// and execution continues past the interrupt site.
pub struct Interrupt {
    resume_payload: Option<Value>,
}

impl Interrupt {
    /// Interrupts the current run with the given payload, or returns the resume value if
    /// this run is a resume.
    ///
    /// On first run, this returns `Err(`[`GraphError::Interrupt`](crate::graph::errors::GraphError::Interrupt)`(payload))`.
    /// Use `?` to propagate it out of the node function.
    ///
    /// On resume, this returns `Ok(resume_value)` where `resume_value` is the value supplied
    /// in [`SessionConfig::resume_payload`](crate::graph::graph::SessionConfig::resume_payload).
    pub fn interrupt(self, payload: impl Into<Value>) -> Result<Value, GraphError> {
        match self.resume_payload {
            Some(v) => Ok(v),
            None => Err(GraphError::Interrupt(payload.into())),
        }
    }
}

impl<N: GraphNode> FromRuntimeContext<N> for Interrupt {
    fn from_rtx(rtx: &RuntimeContext<N>) -> Result<Self, GraphError> {
        Ok(Interrupt {
            resume_payload: rtx.resume_payload.clone(),
        })
    }
}
