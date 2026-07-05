// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    archetype::standalone::fields::{
        spec::{FieldsSpec, IntoFieldSpecs},
        tools::NamedTool,
    },
    context::{ResolvedContext, ResolvedContextProvider},
};

impl IntoFieldSpecs for ResolvedContext {
    fn extend_fields(&self, fields: &mut FieldsSpec) {
        fields.extend_from(&self.runtime);

        for provider in &self.providers {
            match provider {
                ResolvedContextProvider::Anthropic(p) => fields.extend_from(p),
                ResolvedContextProvider::OpenAi(p) => fields.extend_from(p),
                ResolvedContextProvider::Ollama(p) => fields.extend_from(p),
                ResolvedContextProvider::OpenRouter(p) => fields.extend_from(p),
                ResolvedContextProvider::Xai(p) => fields.extend_from(p),
                ResolvedContextProvider::Gemini(p) => fields.extend_from(p),
            }
        }

        fields.extend_from(&self.agent);

        for (name, tool) in &self.tools {
            fields.extend_from(&NamedTool(name.as_str(), tool));
        }

        if let Some(http) = &self.http_server {
            fields.extend_from(http);
        }
    }
}
