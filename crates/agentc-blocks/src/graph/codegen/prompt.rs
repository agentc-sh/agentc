// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

use proc_macro2::TokenStream;
use quote::quote;

use agentc_compiler::generator::{
    blocks::template::TemplateFragment,
    context::GenerationContext,
    errors::GeneratorError,
    extension::{ErasedContributionValue, ExtensionRegistry},
};

use crate::{
    context::{
        ResolvedContext, ResolvedContextAgentPromptMessage, ResolvedContextAgentPromptMessageRole,
        ResolvedContextAgentPromptSource, ResolvedContextAgentPromptSourceLangfuse,
    },
    contributions::dependency::{
        CargoDependencies, CargoDependencyContribution, CargoPatchContribution, CargoPatches,
        RuntimeDependencyContribution,
    },
    fields::FieldsSpec,
};

/// Generates the `PromptSource` argument wired into `with_prompt_source`.
pub struct PromptSourceCodeGen;

impl PromptSourceCodeGen {
    pub fn generate(
        ctx: &ResolvedContext,
        fields: &FieldsSpec,
    ) -> Result<(TokenStream, TokenStream), GeneratorError> {
        match &ctx.agent.prompt {
            None => Ok(Self::constant(&[])),
            Some(ResolvedContextAgentPromptSource::Constant { messages }) => {
                Ok(Self::constant(messages))
            }
            Some(ResolvedContextAgentPromptSource::Langfuse(prompt)) => {
                Self::langfuse(prompt, fields)
            }
        }
    }

    fn constant(messages: &[ResolvedContextAgentPromptMessage]) -> (TokenStream, TokenStream) {
        let imports = quote! {
            use agentc_prompt::{
                source::ConstantPromptSource,
                template::{PromptTemplate, Role},
            };
        };

        if messages.is_empty() {
            return (imports, quote! { ConstantPromptSource::new(PromptTemplate::default()) });
        }

        let parts = messages.iter().map(|message| {
            let role = match message.role {
                ResolvedContextAgentPromptMessageRole::System => quote! { Role::System },
                ResolvedContextAgentPromptMessageRole::User => quote! { Role::User },
                ResolvedContextAgentPromptMessageRole::Assistant => quote! { Role::Assistant },
            };
            let content = &message.content;

            quote! { .with_part(#role, #content) }
        });

        (
            imports,
            quote! {
                ConstantPromptSource::new(
                    PromptTemplate::new()
                        #(#parts)*
                )
            },
        )
    }

    fn langfuse(
        prompt: &ResolvedContextAgentPromptSourceLangfuse,
        fields: &FieldsSpec,
    ) -> Result<(TokenStream, TokenStream), GeneratorError> {
        let prompt_name =
            Self::required_accessor(fields, &["agent", "prompt", "langfuse", "prompt_name"])?;
        let public_key =
            Self::required_accessor(fields, &["agent", "prompt", "langfuse", "public_key"])?;
        let secret_key =
            Self::required_accessor(fields, &["agent", "prompt", "langfuse", "secret_key"])?;
        let mut client_calls = Vec::new();
        let mut source_calls = Vec::new();

        if prompt.base_url.is_some() {
            let base_url =
                Self::required_accessor(fields, &["agent", "prompt", "langfuse", "base_url"])?;

            client_calls.push(quote! {
                .base_url(#base_url.clone())
            });
        }

        if prompt.fetch_timeout_seconds.is_some() {
            let fetch_timeout_seconds = Self::required_accessor(
                fields,
                &["agent", "prompt", "langfuse", "fetch_timeout_seconds"],
            )?;

            client_calls.push(quote! {
                .fetch_timeout(Duration::from_secs(#fetch_timeout_seconds))
            });
        }

        if prompt.max_retries.is_some() {
            let max_retries =
                Self::required_accessor(fields, &["agent", "prompt", "langfuse", "max_retries"])?;

            client_calls.push(quote! {
                .max_retries(#max_retries)
            });
        }

        if prompt.label.is_some() {
            let label = Self::required_accessor(fields, &["agent", "prompt", "langfuse", "label"])?;

            source_calls.push(quote! {
                .label(#label.clone())
            });
        } else if prompt.version.is_some() {
            let version =
                Self::required_accessor(fields, &["agent", "prompt", "langfuse", "version"])?;

            source_calls.push(quote! {
                .version(#version)
            });
        }

        if prompt.cache_ttl_seconds.is_some() {
            let cache_ttl_seconds = Self::required_accessor(
                fields,
                &["agent", "prompt", "langfuse", "cache_ttl_seconds"],
            )?;

            source_calls.push(quote! {
                .cache_ttl(Duration::from_secs(#cache_ttl_seconds))
            });
        }

        Ok((
            quote! {
                use std::time::Duration;

                use agentc_prompt::source::langfuse::{
                    LangfusePromptSource,
                    client::LangfuseClient,
                };
            },
            quote! {
                LangfusePromptSource::builder()
                    .client(
                        LangfuseClient::builder()
                            .public_key(#public_key.clone())
                            .secret_key(#secret_key.clone().as_inner())
                            #(#client_calls)*
                            .build()?
                    )
                    .prompt_name(#prompt_name.clone())
                    #(#source_calls)*
                    .build()?
            },
        ))
    }

    fn required_accessor(
        fields: &FieldsSpec,
        path: &[&str],
    ) -> Result<TokenStream, GeneratorError> {
        fields
            .config_accessor(path)
            .ok_or_else(|| {
                GeneratorError::unexpected(format!(
                    "Missing required generated field '{}'",
                    path.join("."),
                ))
            })
    }
}

pub struct PromptCargoFragment;

impl TemplateFragment<ResolvedContext> for PromptCargoFragment {
    fn generate_contribution(
        &self,
        ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "cargo::dependencies" => {
                let mut dependency =
                    RuntimeDependencyContribution::new("agentc-prompt").feature("tiktoken");

                if matches!(&ctx.agent.prompt, Some(ResolvedContextAgentPromptSource::Langfuse(_)))
                {
                    dependency = dependency.feature("langfuse");
                }

                Ok(ErasedContributionValue::new(
                    CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                        dependency,
                    )])
                    .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
                ))
            }
            "cargo::patches" => Ok(ErasedContributionValue::new(
                CargoPatches::from_entries([CargoPatchContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-prompt"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point,))),
        }
    }

    fn generate_files(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        _registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, String)>, GeneratorError> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::{
        context::{
            ResolvedContextAgent, ResolvedContextAgentModel, ResolvedContextAgentPromptMessage,
            ResolvedContextAgentPromptMessageRole, ResolvedContextRuntime,
        },
        types::RuntimeValue,
    };

    struct PromptCodeGenFixture;

    impl PromptCodeGenFixture {
        fn context(prompt: Option<ResolvedContextAgentPromptSource>) -> ResolvedContext {
            ResolvedContext {
                slug: "assistant".to_string(),
                agent_name: "assistant".to_string(),
                runtime: ResolvedContextRuntime {
                    default_tenant_id: RuntimeValue::constant("default".to_string()),
                },
                providers: vec![],
                agent: ResolvedContextAgent {
                    version: "0.1.0".to_string(),
                    description: None,
                    prompt,
                    capabilities: None,
                    capability_policy: None,
                    model: ResolvedContextAgentModel {
                        provider: RuntimeValue::constant("anthropic".to_string()),
                        name: RuntimeValue::constant("claude".to_string()),
                    },
                },
                blocks: HashMap::new(),
                tools: HashMap::new(),
                skills: HashMap::new(),
                http_server: None,
            }
        }

        fn langfuse(
            label: Option<RuntimeValue<String>>,
            version: Option<RuntimeValue<u32>>,
        ) -> ResolvedContextAgentPromptSource {
            ResolvedContextAgentPromptSource::Langfuse(ResolvedContextAgentPromptSourceLangfuse {
                prompt_name: RuntimeValue::constant("support/assistant".to_string()),
                public_key: RuntimeValue::required_runtime("LANGFUSE_PUBLIC_KEY"),
                secret_key: RuntimeValue::secret_runtime("LANGFUSE_SECRET_KEY"),
                base_url: Some(RuntimeValue::constant("https://cloud.langfuse.com".to_string())),
                label,
                version,
                cache_ttl_seconds: Some(RuntimeValue::constant(30)),
                fetch_timeout_seconds: Some(RuntimeValue::constant(5)),
                max_retries: Some(RuntimeValue::constant(2)),
            })
        }

        fn generate(prompt: Option<ResolvedContextAgentPromptSource>) -> (String, String) {
            let context = Self::context(prompt);
            let fields = FieldsSpec::collect_from(&context.agent);
            let (imports, source) = PromptSourceCodeGen::generate(&context, &fields)
                .expect("prompt source should generate");

            (imports.to_string(), source.to_string())
        }
    }

    #[test]
    fn constant_source_generates_prompt_template() {
        let (imports, source) =
            PromptCodeGenFixture::generate(Some(ResolvedContextAgentPromptSource::Constant {
                messages: vec![ResolvedContextAgentPromptMessage {
                    role: ResolvedContextAgentPromptMessageRole::System,
                    content: "hi {{ agent_name }}".to_string(),
                }],
            }));

        assert!(imports.contains("ConstantPromptSource"));
        assert!(imports.contains("PromptTemplate"));
        assert!(imports.contains("Role"));
        assert!(source.contains("ConstantPromptSource :: new"));
        assert!(source.contains("PromptTemplate :: new"));
        assert!(source.contains("Role :: System"));
        assert!(source.contains("hi {{ agent_name }}"));
    }

    #[test]
    fn langfuse_source_generates_configured_builders() {
        let (imports, source) =
            PromptCodeGenFixture::generate(Some(PromptCodeGenFixture::langfuse(
                Some(RuntimeValue::constant("staging".to_string())),
                None,
            )));

        assert!(imports.contains("std :: time :: Duration"));
        assert!(imports.contains("LangfusePromptSource"));
        assert!(imports.contains("LangfuseClient"));
        assert!(source.contains("LangfusePromptSource :: builder"));
        assert!(source.contains("LangfuseClient :: builder"));
        assert!(source.contains("config . agent . prompt . langfuse . prompt_name"));
        assert!(source.contains("config . agent . prompt . langfuse . public_key"));
        assert!(source.contains("config . agent . prompt . langfuse . secret_key"));
        assert!(source.contains("base_url"));
        assert!(source.contains("fetch_timeout"));
        assert!(source.contains("max_retries"));
        assert!(source.contains("label"));
        assert!(source.contains("cache_ttl"));
    }

    #[test]
    fn langfuse_default_selector_omits_selector_calls() {
        let (_, source) =
            PromptCodeGenFixture::generate(Some(PromptCodeGenFixture::langfuse(None, None)));

        assert!(!source.contains("label"));
        assert!(!source.contains("version"));
    }

    #[test]
    fn prompt_dependency_includes_langfuse_for_langfuse_source() {
        let context = GenerationContext::new(PromptCodeGenFixture::context(Some(
            PromptCodeGenFixture::langfuse(None, None),
        )));
        let dependencies = PromptCargoFragment
            .generate_contribution(&context, "cargo::dependencies")
            .expect("dependency should generate")
            .downcast::<CargoDependencies>()
            .expect("dependency should have the expected type");

        assert_eq!(dependencies.len(), 1);
        assert!(matches!(
            dependencies
                .get(&"agentc-prompt")
                .unwrap(),
            CargoDependencyContribution::Runtime(dependency)
                if dependency.features.len() == 2
                    && dependency.features.contains("langfuse")
                    && dependency.features.contains("tiktoken")
        ));
    }

    #[test]
    fn prompt_dependency_always_includes_tiktoken() {
        let dependencies = PromptCargoFragment
            .generate_contribution(
                &GenerationContext::new(PromptCodeGenFixture::context(None)),
                "cargo::dependencies",
            )
            .expect("dependency should generate")
            .downcast::<CargoDependencies>()
            .expect("dependency should have the expected type");

        assert_eq!(dependencies.len(), 1);
        assert!(matches!(
            dependencies
                .get(&"agentc-prompt")
                .unwrap(),
            CargoDependencyContribution::Runtime(dependency)
                if dependency.features.len() == 1
                    && dependency.features.contains("tiktoken")
        ));
    }

    #[test]
    fn prompt_fragment_contributes_runtime_patch() {
        let patches = PromptCargoFragment
            .generate_contribution(
                &GenerationContext::new(PromptCodeGenFixture::context(None)),
                "cargo::patches",
            )
            .expect("patch should generate")
            .downcast::<CargoPatches>()
            .expect("patch should have the expected type");

        assert_eq!(patches.len(), 1);
        assert!(patches.get(&"agentc-prompt").is_some());
    }
}
