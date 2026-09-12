// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;

use crate::{errors::PromptError, template::PromptTemplate};

/// A source that resolves the agent's prompt template on demand.
///
/// Implementors allow the prompt to originate from a baked-in value, a file, or
/// a remote prompt-management service. `load` is called once per model pass;
/// implementations that reach out over the network handle their own caching.
#[async_trait]
pub trait PromptSource: Send + Sync {
    async fn load(&self) -> Result<PromptTemplate, PromptError>;
}
