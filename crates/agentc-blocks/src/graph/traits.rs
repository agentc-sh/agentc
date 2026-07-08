// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::de::DeserializeOwned;
use serde_json::{Value, from_value};

use crate::{context::ResolvedContext, errors::BlocksError, graph::types::ResolvedGraph};

pub trait AgentGraph: Send + Sync {
    type Config: DeserializeOwned;

    fn name(&self) -> &str;

    fn resolve(
        &self,
        context: ResolvedContext,
        config: Self::Config,
    ) -> Result<ResolvedGraph, BlocksError>;
}

pub trait ErasedAgentGraph: Send + Sync {
    fn name(&self) -> &str;
    fn resolve_erased(
        &self,
        context: ResolvedContext,
        config: Value,
    ) -> Result<ResolvedGraph, BlocksError>;
}

impl<T> ErasedAgentGraph for T
where
    T: AgentGraph + Send + Sync,
    T::Config: 'static,
{
    fn name(&self) -> &str {
        self.name()
    }

    fn resolve_erased(
        &self,
        context: ResolvedContext,
        config: Value,
    ) -> Result<ResolvedGraph, BlocksError> {
        self.resolve(context, from_value(config)?)
    }
}
