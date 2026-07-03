// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use futures::future::BoxFuture;
use std::error::Error;

#[async_trait]
pub trait RepoScope: Send {}

#[async_trait]
pub trait RepoScopeFactory: Send + Sync {
    type Scope<'scope>: RepoScope
    where
        Self: 'scope;
    type Error: Error + Send + Sync + 'static;

    async fn rw_scope<F, R, E>(&self, f: F) -> Result<R, E>
    where
        E: From<Self::Error> + Send,
        F: for<'scope> FnOnce(&'scope mut Self::Scope<'scope>) -> BoxFuture<'scope, Result<R, E>>
            + Send,
        R: Send;

    async fn ro_scope<F, R, E>(&self, f: F) -> Result<R, E>
    where
        E: From<Self::Error> + Send,
        F: for<'scope> FnOnce(&'scope Self::Scope<'scope>) -> BoxFuture<'scope, Result<R, E>>
            + Send,
        R: Send;
}
