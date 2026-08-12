// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use quote::quote;

use agentc_compiler::generator::{
    blocks::fragment::{Fragment, FragmentBlock},
    context::GenerationContext,
    errors::GeneratorError,
    extension::{Contribution, ErasedContributionValue},
};

use crate::{
    config::{
        fields::FieldValue,
        sections::{
            block::ConfigSectionBlockBuilderExt,
            contribution::{ConfigSectionContribution, ConfigSections},
        },
    },
    context::{ResolvedContext, ResolvedContextNetwork},
    contributions::dependency::{
        CargoDependencies, CargoDependencyContribution, CargoPatchContribution, CargoPatches,
        RuntimeDependencyContribution,
    },
};

pub struct NetworkSection;

impl NetworkSection {
    pub const NAME: &'static str = "network";

    pub fn block(id: &'static str) -> FragmentBlock<ResolvedContext> {
        FragmentBlock::<ResolvedContext>::builder()
            .id(id)
            .contribute_config_sections()
            .contribute(Contribution::<CargoDependencies>::strict("cargo::dependencies"))
            .contribute(Contribution::<CargoPatches>::strict("cargo::patches"))
            .build(Self)
    }

    fn leaves(network: &ResolvedContextNetwork) -> Vec<(Vec<String>, FieldValue)> {
        vec![
            (
                vec!["network".into(), "user_agent".into()],
                FieldValue::from(&network.user_agent),
            ),
            (
                vec!["network".into(), "headers".into()],
                FieldValue::from(&network.headers),
            ),
            (
                vec!["network".into(), "limits".into(), "connect_timeout_ms".into()],
                FieldValue::from(&network.limits.connect_timeout_ms),
            ),
            (
                vec!["network".into(), "limits".into(), "read_timeout_ms".into()],
                FieldValue::from(&network.limits.read_timeout_ms),
            ),
            (
                vec!["network".into(), "limits".into(), "request_timeout_ms".into()],
                FieldValue::from(&network.limits.request_timeout_ms),
            ),
            (
                vec!["network".into(), "limits".into(), "max_redirects".into()],
                FieldValue::from(&network.limits.max_redirects),
            ),
            (
                vec!["network".into(), "limits".into(), "max_response_bytes".into()],
                FieldValue::from(&network.limits.max_response_bytes),
            ),
            (
                vec!["network".into(), "limits".into(), "concurrency_limit".into()],
                FieldValue::from(&network.limits.concurrency_limit),
            ),
            (
                vec![
                    "network".into(),
                    "policy".into(),
                    "addresses".into(),
                    "allow_loopback".into(),
                ],
                FieldValue::from(&network.policy.addresses.allow_loopback),
            ),
            (
                vec![
                    "network".into(),
                    "policy".into(),
                    "addresses".into(),
                    "allow_private".into(),
                ],
                FieldValue::from(&network.policy.addresses.allow_private),
            ),
            (
                vec![
                    "network".into(),
                    "policy".into(),
                    "addresses".into(),
                    "allow_link_local".into(),
                ],
                FieldValue::from(&network.policy.addresses.allow_link_local),
            ),
            (
                vec!["network".into(), "policy".into(), "methods".into()],
                FieldValue::from(&network.policy.methods),
            ),
            (
                vec!["network".into(), "policy".into(), "allow".into()],
                FieldValue::from(&network.policy.allow),
            ),
        ]
    }

    fn section(&self, network: &ResolvedContextNetwork) -> Result<ConfigSections, GeneratorError> {
        let mut constants = Vec::new();
        let mut defaults = Vec::new();
        let mut field_mappings = Vec::new();

        for (path, value) in &Self::leaves(network) {
            let tokens = value.loader_tokens(path);

            constants.extend(tokens.constant);
            defaults.extend(tokens.default);
            field_mappings.extend(tokens.field);
        }

        ConfigSections::from_entries([ConfigSectionContribution::new(Self::NAME)
            .uses(quote! {
                use agentc_http::{
                    client::{
                        HttpClient,
                        HttpClientBuilder,
                        errors::HttpClientError,
                        policies::{MethodPolicy, PatternPolicy, PublicAddressPolicy, UrlPattern},
                    },
                    protocol::Method,
                };
            })
            .types(quote! {
                #[derive(Debug, Clone, Serialize, Deserialize, Default)]
                #[serde(default)]
                pub struct ConfigNetwork {
                    pub user_agent: Option<String>,
                    pub headers: std::collections::BTreeMap<String, String>,
                    pub limits: ConfigNetworkLimits,
                    pub policy: ConfigNetworkPolicy,
                }

                impl ConfigNetwork {
                    /// Returns an unbuilt client so each consumer can build on its own runtime.
                    pub fn builder(&self) -> Result<HttpClientBuilder, HttpClientError> {
                        let mut builder = HttpClient::builder()
                            .max_redirects(self.limits.max_redirects);

                        if let Some(user_agent) = &self.user_agent {
                            builder = builder.user_agent(user_agent.clone());
                        }

                        for (name, value) in &self.headers {
                            builder = builder.header(name.clone(), value.clone());
                        }

                        if let Some(timeout_ms) = self.limits.connect_timeout_ms {
                            builder = builder.connect_timeout(
                                std::time::Duration::from_millis(timeout_ms),
                            );
                        }

                        if let Some(timeout_ms) = self.limits.read_timeout_ms {
                            builder = builder.read_timeout(
                                std::time::Duration::from_millis(timeout_ms),
                            );
                        }

                        if let Some(timeout_ms) = self.limits.request_timeout_ms {
                            builder = builder.request_timeout(
                                std::time::Duration::from_millis(timeout_ms),
                            );
                        }

                        if let Some(max_bytes) = self.limits.max_response_bytes {
                            builder = builder.max_response_bytes(max_bytes);
                        }

                        if let Some(permits) = self.limits.concurrency_limit {
                            builder = builder.concurrency_limit(permits);
                        }

                        let mut addresses = PublicAddressPolicy::default();

                        if self.policy.addresses.allow_loopback {
                            addresses = addresses.allow_loopback();
                        }

                        if self.policy.addresses.allow_private {
                            addresses = addresses.allow_private();
                        }

                        if self.policy.addresses.allow_link_local {
                            addresses = addresses.allow_link_local();
                        }

                        builder = builder.policy(addresses);

                        if let Some(methods) = &self.policy.methods {
                            builder = builder.policy(MethodPolicy::allow(
                                methods
                                    .iter()
                                    .map(|method| Method::from_bytes(method.as_bytes()))
                                    .collect::<Result<Vec<_>, _>>()
                                    .map_err(|error| {
                                        HttpClientError::configuration(error.to_string())
                                    })?,
                            ));
                        }

                        if !self.policy.allow.is_empty() {
                            builder = builder.policy(
                                PatternPolicy::allow(
                                    self.policy
                                        .allow
                                        .iter()
                                        .map(|pattern| UrlPattern {
                                            protocol: pattern.protocol.clone(),
                                            hostname: pattern.hostname.clone(),
                                            port: pattern.port.clone(),
                                            pathname: pattern.pathname.clone(),
                                            ..Default::default()
                                        })
                                        .collect::<Vec<_>>(),
                                )?,
                            );
                        }

                        Ok(builder)
                    }
                }

                #[derive(Debug, Clone, Serialize, Deserialize)]
                #[serde(default)]
                pub struct ConfigNetworkLimits {
                    pub connect_timeout_ms: Option<u64>,
                    pub read_timeout_ms: Option<u64>,
                    pub request_timeout_ms: Option<u64>,
                    pub max_redirects: usize,
                    pub max_response_bytes: Option<u64>,
                    pub concurrency_limit: Option<usize>,
                }

                impl Default for ConfigNetworkLimits {
                    fn default() -> Self {
                        ConfigNetworkLimits {
                            connect_timeout_ms: None,
                            read_timeout_ms: None,
                            request_timeout_ms: None,
                            max_redirects: 5,
                            max_response_bytes: None,
                            concurrency_limit: None,
                        }
                    }
                }

                #[derive(Debug, Clone, Serialize, Deserialize, Default)]
                #[serde(default)]
                pub struct ConfigNetworkPolicy {
                    pub addresses: ConfigNetworkPolicyAddresses,
                    pub methods: Option<std::collections::BTreeSet<String>>,
                    pub allow: Vec<ConfigNetworkUrlPattern>,
                }

                #[derive(Debug, Clone, Serialize, Deserialize, Default)]
                #[serde(default)]
                pub struct ConfigNetworkPolicyAddresses {
                    pub allow_loopback: bool,
                    pub allow_private: bool,
                    pub allow_link_local: bool,
                }

                #[derive(Debug, Clone, Serialize, Deserialize, Default)]
                #[serde(default)]
                pub struct ConfigNetworkUrlPattern {
                    pub protocol: Option<String>,
                    pub hostname: Option<String>,
                    pub port: Option<String>,
                    pub pathname: Option<String>,
                }
            })
            .fields(quote! {
                pub network: ConfigNetwork,
            })
            .loader(quote! {
                #(#constants)*
                #(#defaults)*
            })
            .mapper(quote! {
                #(#field_mappings)*
            })])
        .map_err(|error| GeneratorError::unexpected(error.to_string()))
    }
}

impl Fragment<ResolvedContext> for NetworkSection {
    fn generate_contribution(
        &self,
        ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "config::sections::use"
            | "config::sections::types"
            | "config::sections::fields"
            | "config::sections::loader"
            | "config::sections::mapper" => {
                Ok(ErasedContributionValue::new(self.section(&ctx.network)?))
            }
            "cargo::dependencies" => Ok(ErasedContributionValue::new(
                CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-http")
                        .default_features(false)
                        .feature("client"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            "cargo::patches" => Ok(ErasedContributionValue::new(
                CargoPatches::from_entries([CargoPatchContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-http"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    use crate::types::RuntimeValue;

    fn context() -> GenerationContext<ResolvedContext> {
        GenerationContext::new(
            serde_json::from_value(json!({
                "slug": "assistant",
                "agent_name": "assistant",
                "runtime": { "default_tenant_id": "default" },
                "providers": [],
                "agent": {
                    "version": "0.1.0",
                    "description": null,
                    "prompt": null,
                    "capabilities": null,
                    "capability_policy": null,
                    "model": { "provider": "anthropic", "name": "claude" }
                },
                "blocks": {},
                "tools": {},
                "skills": {},
                "http_server": null
            }))
            .unwrap(),
        )
    }

    #[test]
    fn contributes_the_http_client_dependency_unconditionally() {
        let dependencies = NetworkSection
            .generate_contribution(&context(), "cargo::dependencies")
            .unwrap()
            .downcast::<CargoDependencies>()
            .unwrap();

        assert_eq!(dependencies.len(), 1);
        assert!(matches!(
            dependencies
                .get(&"agentc-http")
                .unwrap(),
            CargoDependencyContribution::Runtime(dependency)
                if dependency.default_features == Some(false)
                    && dependency.features.len() == 1
                    && dependency.features.contains("client")
        ));
    }

    #[test]
    fn the_section_defines_network_config_and_its_field() {
        let sections = NetworkSection
            .generate_contribution(&context(), "config::sections::types")
            .unwrap()
            .downcast::<ConfigSections>()
            .unwrap();
        let section = sections
            .get(&NetworkSection::NAME)
            .expect("network section is contributed");

        assert!(
            section
                .types
                .as_str()
                .contains("pub struct ConfigNetwork")
        );
        assert!(
            section
                .types
                .as_str()
                .contains("fn builder (& self) -> Result < HttpClientBuilder , HttpClientError >")
        );
        assert!(
            section
                .fields
                .as_str()
                .contains("pub network : ConfigNetwork")
        );
    }

    #[test]
    fn a_locked_leaf_emits_a_constant_and_no_field_mapping() {
        let mut network = ResolvedContextNetwork::default();
        network.limits.max_redirects = RuntimeValue::constant(3);

        let sections = NetworkSection
            .section(&network)
            .unwrap();
        let section = sections
            .get(&NetworkSection::NAME)
            .unwrap();

        assert!(
            section
                .loader
                .as_str()
                .contains(r#". constant (path ! ["network" , "limits" , "max_redirects"] , serde_json :: json ! (3))"#)
        );
        assert!(
            !section
                .mapper
                .as_str()
                .contains("NETWORK_MAX_REDIRECTS")
        );
    }

    #[test]
    fn a_runtime_leaf_with_a_default_emits_both_a_default_and_a_field_mapping() {
        let mut network = ResolvedContextNetwork::default();
        network.limits.max_redirects =
            RuntimeValue::default_runtime("NETWORK_MAX_REDIRECTS", 5usize);

        let sections = NetworkSection
            .section(&network)
            .unwrap();
        let section = sections
            .get(&NetworkSection::NAME)
            .unwrap();

        assert!(
            section
                .mapper
                .as_str()
                .contains(r#". field (path ! ["network" , "limits" , "max_redirects"] , "NETWORK_MAX_REDIRECTS")"#)
        );
        assert!(
            section
                .loader
                .as_str()
                .contains(r#". default (path ! ["network" , "limits" , "max_redirects"] , serde_json :: json ! (5))"#)
        );
    }
}
