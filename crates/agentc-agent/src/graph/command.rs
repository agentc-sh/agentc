// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::graph::state::{GraphNode, UpdateOf};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GraphTransition<N> {
    Node(N),
    End,
}

pub struct GraphNodeCommand<N>
where
    N: GraphNode,
{
    pub goto: Option<GraphTransition<N>>,
    pub update: Option<UpdateOf<N>>,
}

impl<N> GraphNodeCommand<N>
where
    N: GraphNode,
{
    pub fn new(goto: Option<GraphTransition<N>>, update: Option<UpdateOf<N>>) -> Self {
        Self { goto, update }
    }

    pub fn goto(node: N) -> Self {
        Self {
            goto: Some(GraphTransition::Node(node)),
            update: None,
        }
    }

    pub fn goto_and_update(node: N, update: UpdateOf<N>) -> Self {
        Self {
            goto: Some(GraphTransition::Node(node)),
            update: Some(update),
        }
    }

    pub fn end() -> Self {
        Self {
            goto: Some(GraphTransition::End),
            update: None,
        }
    }

    pub fn end_and_update(update: UpdateOf<N>) -> Self {
        Self {
            goto: Some(GraphTransition::End),
            update: Some(update),
        }
    }
}
