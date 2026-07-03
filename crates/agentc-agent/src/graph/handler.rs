// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::{future::Future, marker::PhantomData};

use crate::graph::{
    command::GraphNodeCommand,
    context::{FromRuntimeContext, RuntimeContext},
    errors::GraphError,
    state::GraphNode,
};

#[async_trait]
pub trait GraphNodeHandler<N>: Send + Sync
where
    N: GraphNode,
{
    async fn handle(&self, rtx: &RuntimeContext<N>) -> Result<GraphNodeCommand<N>, GraphError>;
}

#[async_trait]
pub trait GraphNodeFunction<N, Args>: Send + Sync
where
    N: GraphNode,
{
    async fn call(&self, rtx: &RuntimeContext<N>) -> Result<GraphNodeCommand<N>, GraphError>;
}

macro_rules! impl_graph_node_fn {
    ($($arg:ident),*) => {
        #[async_trait]
        impl<N, $($arg,)* F, Fut> GraphNodeFunction<N, ($($arg,)*)> for F
        where
            N: GraphNode,
            $($arg: FromRuntimeContext<N> + Send,)*
            F: Fn($($arg),*) -> Fut + Send + Sync,
            Fut: Future<Output = Result<GraphNodeCommand<N>, GraphError>> + Send,
        {
            async fn call(&self, rtx: &RuntimeContext<N>) -> Result<GraphNodeCommand<N>, GraphError> {
                (self)($($arg::from_rtx(rtx)?),*).await
            }
        }
    };
}

impl_graph_node_fn!(A1);
impl_graph_node_fn!(A1, A2);
impl_graph_node_fn!(A1, A2, A3);
impl_graph_node_fn!(A1, A2, A3, A4);
impl_graph_node_fn!(A1, A2, A3, A4, A5);
impl_graph_node_fn!(A1, A2, A3, A4, A5, A6);
impl_graph_node_fn!(A1, A2, A3, A4, A5, A6, A7);
impl_graph_node_fn!(A1, A2, A3, A4, A5, A6, A7, A8);
impl_graph_node_fn!(A1, A2, A3, A4, A5, A6, A7, A8, A9);
impl_graph_node_fn!(A1, A2, A3, A4, A5, A6, A7, A8, A9, A10);

pub struct GraphNodeHandlerFn<N, F, Args>
where
    N: GraphNode,
{
    func: F,
    _marker: PhantomData<fn() -> (N, Args)>,
}

impl<N, F, Args> GraphNodeHandlerFn<N, F, Args>
where
    N: GraphNode,
    F: GraphNodeFunction<N, Args>,
{
    pub fn new(func: F) -> Self {
        Self { func, _marker: PhantomData }
    }
}

#[async_trait]
impl<N, F, Args> GraphNodeHandler<N> for GraphNodeHandlerFn<N, F, Args>
where
    N: GraphNode,
    F: GraphNodeFunction<N, Args>,
{
    async fn handle(&self, rtx: &RuntimeContext<N>) -> Result<GraphNodeCommand<N>, GraphError> {
        self.func.call(rtx).await
    }
}
