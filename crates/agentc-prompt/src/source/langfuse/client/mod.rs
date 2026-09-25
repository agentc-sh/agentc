// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

mod builder;
mod cache;
mod error;
mod prompts;
mod transport;
mod types;
mod wire;

pub use builder::{
    DEFAULT_BASE_URL, DEFAULT_FETCH_TIMEOUT, DEFAULT_MAX_RETRIES, DEFAULT_PROMPT_CACHE_CAPACITY,
    DEFAULT_PROMPT_CACHE_TTL, LangfuseClientBuilder,
};
pub use error::LangfuseError;
pub use prompts::Prompts;
pub use types::{
    ChatMessage, ChatPrompt, ChatPromptItem, GetPromptRequest, MessagePlaceholder, Prompt,
    PromptCacheMode, PromptMetadata, PromptSelector, TextPrompt,
};

use std::sync::Arc;

use cache::PromptStore;

#[derive(Clone)]
pub struct LangfuseClient {
    inner: Arc<LangfuseClientInner>,
}

struct LangfuseClientInner {
    prompts: PromptStore,
}

impl LangfuseClient {
    pub fn builder() -> LangfuseClientBuilder {
        LangfuseClientBuilder::default()
    }

    pub fn prompts(&self) -> Prompts<'_> {
        Prompts::new(&self.inner.prompts)
    }

    fn new(prompts: PromptStore) -> Self {
        Self {
            inner: Arc::new(LangfuseClientInner { prompts }),
        }
    }
}
