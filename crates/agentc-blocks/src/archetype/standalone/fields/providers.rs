// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    archetype::standalone::fields::spec::{FieldsSpec, IntoFieldSpecs},
    context::{
        ResolvedContextProviderAnthropic,
        ResolvedContextProviderGemini,
        ResolvedContextProviderOllama,
        ResolvedContextProviderOpenAi,
        ResolvedContextProviderOpenRouter,
        ResolvedContextProviderParams,
        ResolvedContextProviderXai,
    },
};

fn model_slug(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

fn push_params(
    fields: &mut FieldsSpec,
    provider: &str,
    slug: &str,
    params: &ResolvedContextProviderParams,
) {
    if let Some(v) = &params.max_tokens {
        fields.push(&["provider", provider, slug, "max_tokens"], v);
    }
    if let Some(v) = &params.temperature {
        fields.push(&["provider", provider, slug, "temperature"], v);
    }
    if let Some(v) = &params.top_p {
        fields.push(&["provider", provider, slug, "top_p"], v);
    }
    if let Some(v) = &params.top_k {
        fields.push(&["provider", provider, slug, "top_k"], v);
    }
    if let Some(v) = &params.stop_sequences {
        fields.push(&["provider", provider, slug, "stop_sequences"], v);
    }
    if let Some(v) = &params.frequency_penalty {
        fields.push(&["provider", provider, slug, "frequency_penalty"], v);
    }
    if let Some(v) = &params.presence_penalty {
        fields.push(&["provider", provider, slug, "presence_penalty"], v);
    }
    if let Some(v) = &params.seed {
        fields.push(&["provider", provider, slug, "seed"], v);
    }
    if let Some(v) = &params.provider_params {
        fields.push(&["provider", provider, slug, "provider_params"], v);
    }
}

impl IntoFieldSpecs for ResolvedContextProviderAnthropic {
    fn extend_fields(&self, fields: &mut FieldsSpec) {
        if let Some(config) = &self.config {
            if let Some(v) = &config.api_key {
                fields.push(&["provider", "anthropic", "api_key"], v);
            }
            if let Some(v) = &config.base_url {
                fields.push(&["provider", "anthropic", "base_url"], v);
            }
        }

        if let Some(params) = &self.params {
            push_params(fields, "anthropic", "params", params);
        }

        if let Some(models) = &self.models {
            for model in models {
                if let Some(params) = &model.params {
                    push_params(fields, "anthropic", model_slug(&model.name).as_str(), params);
                }
            }
        }
    }
}

impl IntoFieldSpecs for ResolvedContextProviderOpenAi {
    fn extend_fields(&self, fields: &mut FieldsSpec) {
        if let Some(config) = &self.config {
            if let Some(v) = &config.api_key {
                fields.push(&["provider", "openai", "api_key"], v);
            }
            if let Some(v) = &config.base_url {
                fields.push(&["provider", "openai", "base_url"], v);
            }
        }

        if let Some(params) = &self.params {
            push_params(fields, "openai", "params", params);
        }

        if let Some(models) = &self.models {
            for model in models {
                if let Some(params) = &model.params {
                    push_params(fields, "openai", model_slug(&model.name).as_str(), params);
                }
            }
        }
    }
}

impl IntoFieldSpecs for ResolvedContextProviderOllama {
    fn extend_fields(&self, fields: &mut FieldsSpec) {
        if let Some(config) = &self.config
            && let Some(v) = &config.base_url
        {
            fields.push(&["provider", "ollama", "base_url"], v);
        }

        if let Some(params) = &self.params {
            push_params(fields, "ollama", "params", params);
        }

        if let Some(models) = &self.models {
            for model in models {
                if let Some(params) = &model.params {
                    push_params(fields, "ollama", model_slug(&model.name).as_str(), params);
                }
            }
        }
    }
}

impl IntoFieldSpecs for ResolvedContextProviderOpenRouter {
    fn extend_fields(&self, fields: &mut FieldsSpec) {
        if let Some(config) = &self.config {
            if let Some(v) = &config.api_key {
                fields.push(&["provider", "openrouter", "api_key"], v);
            }
        }

        if let Some(params) = &self.params {
            push_params(fields, "openrouter", "params", params);
        }

        if let Some(models) = &self.models {
            for model in models {
                if let Some(params) = &model.params {
                    push_params(fields, "openrouter", model_slug(&model.name).as_str(), params);
                }
            }
        }
    }
}

impl IntoFieldSpecs for ResolvedContextProviderXai {
    fn extend_fields(&self, fields: &mut FieldsSpec) {
        if let Some(config) = &self.config {
            if let Some(v) = &config.api_key {
                fields.push(&["provider", "xai", "api_key"], v);
            }
        }

        if let Some(params) = &self.params {
            push_params(fields, "xai", "params", params);
        }

        if let Some(models) = &self.models {
            for model in models {
                if let Some(params) = &model.params {
                    push_params(fields, "xai", model_slug(&model.name).as_str(), params);
                }
            }
        }
    }
}

impl IntoFieldSpecs for ResolvedContextProviderGemini {
    fn extend_fields(&self, fields: &mut FieldsSpec) {
        if let Some(config) = &self.config {
            if let Some(v) = &config.api_key {
                fields.push(&["provider", "gemini", "api_key"], v);
            }
        }

        if let Some(params) = &self.params {
            push_params(fields, "gemini", "params", params);
        }

        if let Some(models) = &self.models {
            for model in models {
                if let Some(params) = &model.params {
                    push_params(fields, "gemini", model_slug(&model.name).as_str(), params);
                }
            }
        }
    }
}
