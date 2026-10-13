// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use quote::quote;

use agentc_compiler::generator::{
    blocks::fragment::{Fragment, FragmentBlock},
    context::GenerationContext,
    errors::GeneratorError,
    extension::ErasedContributionValue,
};

use crate::{
    config::{
        fields::{AsFieldValue, FieldTokens},
        sections::{
            block::ConfigSectionBlockBuilderExt,
            contribution::{ConfigSectionContribution, ConfigSections},
        },
    },
    context::{ResolvedContext, ResolvedContextToolKind},
};

pub struct McpSection;

impl McpSection {
    pub const NAME: &'static str = "mcp";

    pub fn block(id: &'static str) -> FragmentBlock<ResolvedContext> {
        FragmentBlock::<ResolvedContext>::builder()
            .id(id)
            .contribute_config_sections()
            .build(Self)
    }

    fn section(&self, ctx: &ResolvedContext) -> Result<ConfigSections, GeneratorError> {
        let tokens = Self::tokens(ctx);
        let loader = tokens.loader();
        let mapper = tokens.mapper();

        ConfigSections::from_entries([ConfigSectionContribution::new(Self::NAME)
            .types(quote! {
                #[derive(Debug, Clone, Serialize, Deserialize)]
                #[serde(tag = "type", rename_all = "snake_case")]
                pub enum ConfigMcpTransport {
                    Stdio {
                        command: String,
                        #[serde(default)]
                        args: Vec<String>,
                        #[serde(default)]
                        env: HashMap<String, String>,
                    },
                    Http {
                        url: String,
                        #[serde(default)]
                        auth_token: Option<agentc_config::secret::Secret<String>>,
                        #[serde(default)]
                        headers: HashMap<String, String>,
                    },
                }

                #[derive(Debug, Clone, Serialize, Deserialize, Default)]
                #[serde(default)]
                pub struct ConfigMcp {
                    pub servers: HashMap<String, ConfigMcpTransport>,
                }
            })
            .fields(quote! {
                pub mcp: ConfigMcp,
            })
            .loader(quote! {
                #(#loader)*
            })
            .mapper(quote! {
                #(#mapper)*
            })])
        .map_err(|error| GeneratorError::unexpected(error.to_string()))
    }

    fn tokens(ctx: &ResolvedContext) -> FieldTokens {
        let mut tokens = FieldTokens::default();

        for (name, tool) in &ctx.tools {
            let ResolvedContextToolKind::Mcp(mcp) = &tool.kind else {
                continue;
            };

            tokens.extend(mcp.transport.as_field_value().tokens(&[
                "mcp".into(),
                "servers".into(),
                name.as_str().into(),
            ]));
        }

        tokens
    }
}

impl Fragment<ResolvedContext> for McpSection {
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
                Ok(ErasedContributionValue::new(self.section(ctx.as_inner())?))
            }
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }
}
