// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::Value;

/// A source of additional variables contributed into a prompt template render.
///
/// Implementors return a JSON object whose keys are merged into the
/// [`PromptContext`](crate::env::PromptContext) before the template is rendered.
/// If a key conflicts with an existing variable in the context, the contributed
/// value replaces it.
///
/// The method is async to support implementations that need to fetch data at
/// contribution time.
#[async_trait]
pub trait TemplateVars: Send + Sync {
    async fn template_vars(&self) -> Result<Value, TemplateVarsError>;
}

/// An error returned by a [`TemplateVars`] contributor.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct TemplateVarsError(String);

impl TemplateVarsError {
    pub fn custom(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}
