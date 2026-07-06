// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::de::DeserializeOwned;
use serde_json::{Value, from_value};

use crate::{context::ResolvedContext, errors::BlocksError, protocol::types::ResolvedProtocol};

pub trait Protocol: Send + Sync {
    type Config: DeserializeOwned;

    fn name(&self) -> &str;

    fn resolve(
        &self,
        context: ResolvedContext,
        config: Self::Config,
    ) -> Result<ResolvedProtocol, BlocksError>;
}

pub trait ErasedProtocol: Send + Sync {
    fn name(&self) -> &str;
    fn resolve_erased(
        &self,
        context: ResolvedContext,
        config: Value,
    ) -> Result<ResolvedProtocol, BlocksError>;
}

impl<T> ErasedProtocol for T
where
    T: Protocol + Send + Sync,
    T::Config: 'static,
{
    fn name(&self) -> &str {
        self.name()
    }

    fn resolve_erased(
        &self,
        context: ResolvedContext,
        config: Value,
    ) -> Result<ResolvedProtocol, BlocksError> {
        self.resolve(context, from_value(config)?)
    }
}
