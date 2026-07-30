// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    context::{ResolvedContextAgent, ResolvedContextAgentPromptSource},
    fields::spec::{FieldsSpec, IntoFieldSpecs},
};

impl IntoFieldSpecs for ResolvedContextAgent {
    fn extend_fields(&self, fields: &mut FieldsSpec) {
        fields.push(&["agent", "model", "provider"], &self.model.provider);
        fields.push(&["agent", "model", "name"], &self.model.name);

        if let Some(capabilities) = &self.capabilities {
            fields.push(&["agent", "capabilities"], capabilities);
        }

        if let Some(ResolvedContextAgentPromptSource::Langfuse(prompt)) = &self.prompt {
            fields.push(&["agent", "prompt", "langfuse", "prompt_name"], &prompt.prompt_name);
            fields.push(&["agent", "prompt", "langfuse", "public_key"], &prompt.public_key);
            fields.push(&["agent", "prompt", "langfuse", "secret_key"], &prompt.secret_key);

            if let Some(base_url) = &prompt.base_url {
                fields.push(&["agent", "prompt", "langfuse", "base_url"], base_url);
            }

            if let Some(label) = &prompt.label {
                fields.push(&["agent", "prompt", "langfuse", "label"], label);
            }

            if let Some(version) = &prompt.version {
                fields.push(&["agent", "prompt", "langfuse", "version"], version);
            }

            if let Some(cache_ttl_seconds) = &prompt.cache_ttl_seconds {
                fields
                    .push(&["agent", "prompt", "langfuse", "cache_ttl_seconds"], cache_ttl_seconds);
            }

            if let Some(fetch_timeout_seconds) = &prompt.fetch_timeout_seconds {
                fields.push(
                    &["agent", "prompt", "langfuse", "fetch_timeout_seconds"],
                    fetch_timeout_seconds,
                );
            }

            if let Some(max_retries) = &prompt.max_retries {
                fields.push(&["agent", "prompt", "langfuse", "max_retries"], max_retries);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        context::{ResolvedContextAgentModel, ResolvedContextAgentPromptSourceLangfuse},
        types::RuntimeValue,
    };

    struct AgentFixture;

    impl AgentFixture {
        fn agent(
            prompt: Option<ResolvedContextAgentPromptSource>,
            capabilities: Option<RuntimeValue<Vec<String>>>,
        ) -> ResolvedContextAgent {
            ResolvedContextAgent {
                version: "1.0.0".to_string(),
                description: None,
                prompt,
                capabilities,
                capability_policy: None,
                model: ResolvedContextAgentModel {
                    provider: RuntimeValue::constant("anthropic".to_string()),
                    name: RuntimeValue::constant("claude".to_string()),
                },
            }
        }
    }

    #[test]
    fn always_registers_the_model_provider_and_name() {
        let fields = FieldsSpec::collect_from(&AgentFixture::agent(None, None));

        assert!(
            fields
                .get(&["agent", "model", "provider"])
                .is_some()
        );
        assert!(
            fields
                .get(&["agent", "model", "name"])
                .is_some()
        );
        assert!(
            fields
                .get(&["agent", "capabilities"])
                .is_none()
        );
    }

    #[test]
    fn registers_capabilities_only_when_present() {
        let fields = FieldsSpec::collect_from(&AgentFixture::agent(
            None,
            Some(RuntimeValue::constant(vec!["network".to_string()])),
        ));

        assert!(
            fields
                .get(&["agent", "capabilities"])
                .is_some()
        );
    }

    #[test]
    fn registers_present_langfuse_prompt_fields() {
        let fields = FieldsSpec::collect_from(&AgentFixture::agent(
            Some(ResolvedContextAgentPromptSource::Langfuse(
                ResolvedContextAgentPromptSourceLangfuse {
                    prompt_name: RuntimeValue::constant("assistant".to_string()),
                    public_key: RuntimeValue::required_runtime("LANGFUSE_PUBLIC_KEY"),
                    secret_key: RuntimeValue::secret_runtime("LANGFUSE_SECRET_KEY"),
                    base_url: Some(RuntimeValue::constant(
                        "https://cloud.langfuse.com".to_string(),
                    )),
                    label: Some(RuntimeValue::constant("staging".to_string())),
                    version: None,
                    cache_ttl_seconds: Some(RuntimeValue::constant(30)),
                    fetch_timeout_seconds: None,
                    max_retries: Some(RuntimeValue::constant(2)),
                },
            )),
            None,
        ));

        for path in [
            &["agent", "prompt", "langfuse", "prompt_name"][..],
            &["agent", "prompt", "langfuse", "public_key"][..],
            &["agent", "prompt", "langfuse", "secret_key"][..],
            &["agent", "prompt", "langfuse", "base_url"][..],
            &["agent", "prompt", "langfuse", "label"][..],
            &["agent", "prompt", "langfuse", "cache_ttl_seconds"][..],
            &["agent", "prompt", "langfuse", "max_retries"][..],
        ] {
            assert!(fields.get(path).is_some());
        }

        assert!(
            fields
                .get(&["agent", "prompt", "langfuse", "version"])
                .is_none()
        );
        assert!(
            fields
                .get(&["agent", "prompt", "langfuse", "fetch_timeout_seconds"])
                .is_none()
        );
    }
}
