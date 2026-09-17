// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;

use crate::{errors::PromptError, source::traits::PromptSource, template::PromptTemplate};

/// A [`PromptSource`] that always yields a fixed, in-memory template.
pub struct ConstantPromptSource(PromptTemplate);

impl ConstantPromptSource {
    pub fn new(template: impl Into<PromptTemplate>) -> Self {
        Self(template.into())
    }
}

#[async_trait]
impl PromptSource for ConstantPromptSource {
    async fn load(&self) -> Result<PromptTemplate, PromptError> {
        Ok(self.0.clone())
    }
}
