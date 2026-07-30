// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use super::{
    cache::PromptStore,
    error::LangfuseError,
    types::{GetPromptRequest, Prompt, PromptSelector},
};

/// Prompt operations exposed by a [`LangfuseClient`](super::LangfuseClient).
pub struct Prompts<'a> {
    store: &'a PromptStore,
}

impl<'a> Prompts<'a> {
    pub(super) fn new(store: &'a PromptStore) -> Self {
        Self { store }
    }

    pub async fn get(
        &self,
        name: impl Into<String>,
        request: GetPromptRequest,
    ) -> Result<Prompt, LangfuseError> {
        self.store
            .get(name, request)
            .await
    }

    pub async fn invalidate(
        &self,
        name: impl Into<String>,
        selector: PromptSelector,
    ) {
        self.store
            .invalidate(name, selector)
            .await;
    }

    pub async fn invalidate_name(&self, name: &str) {
        self.store
            .invalidate_name(name)
            .await;
    }
}
