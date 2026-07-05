// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_compiler::generator::blocks::codegen::ToIdent;

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

/// Registers every set inference parameter under `["provider", provider, slug, <field>]`.
///
/// The `slug` is either `"params"` for provider-level defaults or a model's slug for
/// per-model overrides. Kept as an extension trait so the provider impls below stay
/// focused on their own config shape.
trait ExtendParamFields {
    fn extend_param_fields(&self, fields: &mut FieldsSpec, provider: &str, slug: &str);
}

impl ExtendParamFields for ResolvedContextProviderParams {
    fn extend_param_fields(&self, fields: &mut FieldsSpec, provider: &str, slug: &str) {
        if let Some(v) = &self.max_tokens {
            fields.push(&["provider", provider, slug, "max_tokens"], v);
        }
        if let Some(v) = &self.temperature {
            fields.push(&["provider", provider, slug, "temperature"], v);
        }
        if let Some(v) = &self.top_p {
            fields.push(&["provider", provider, slug, "top_p"], v);
        }
        if let Some(v) = &self.top_k {
            fields.push(&["provider", provider, slug, "top_k"], v);
        }
        if let Some(v) = &self.stop_sequences {
            fields.push(&["provider", provider, slug, "stop_sequences"], v);
        }
        if let Some(v) = &self.frequency_penalty {
            fields.push(&["provider", provider, slug, "frequency_penalty"], v);
        }
        if let Some(v) = &self.presence_penalty {
            fields.push(&["provider", provider, slug, "presence_penalty"], v);
        }
        if let Some(v) = &self.seed {
            fields.push(&["provider", provider, slug, "seed"], v);
        }
        if let Some(v) = &self.provider_params {
            fields.push(&["provider", provider, slug, "provider_params"], v);
        }
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
            params.extend_param_fields(fields, "anthropic", "params");
        }

        if let Some(models) = &self.models {
            for model in models {
                if let Some(params) = &model.params {
                    params.extend_param_fields(fields, "anthropic", model.name.to_ident().as_str());
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
            params.extend_param_fields(fields, "openai", "params");
        }

        if let Some(models) = &self.models {
            for model in models {
                if let Some(params) = &model.params {
                    params.extend_param_fields(fields, "openai", model.name.to_ident().as_str());
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
            params.extend_param_fields(fields, "ollama", "params");
        }

        if let Some(models) = &self.models {
            for model in models {
                if let Some(params) = &model.params {
                    params.extend_param_fields(fields, "ollama", model.name.to_ident().as_str());
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
            params.extend_param_fields(fields, "openrouter", "params");
        }

        if let Some(models) = &self.models {
            for model in models {
                if let Some(params) = &model.params {
                    params.extend_param_fields(fields, "openrouter", model.name.to_ident().as_str());
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
            params.extend_param_fields(fields, "xai", "params");
        }

        if let Some(models) = &self.models {
            for model in models {
                if let Some(params) = &model.params {
                    params.extend_param_fields(fields, "xai", model.name.to_ident().as_str());
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
            params.extend_param_fields(fields, "gemini", "params");
        }

        if let Some(models) = &self.models {
            for model in models {
                if let Some(params) = &model.params {
                    params.extend_param_fields(fields, "gemini", model.name.to_ident().as_str());
                }
            }
        }
    }
}
