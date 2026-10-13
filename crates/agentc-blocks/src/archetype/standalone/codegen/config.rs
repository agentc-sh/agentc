// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::{collections::BTreeMap, iter::once, path::PathBuf};

use agentc_compiler::generator::{
    blocks::codegen::CodeGen, context::GenerationContext, errors::GeneratorError,
    extension::ExtensionRegistry,
};

use crate::{
    config::fields::{FieldSpec, FieldTokens, FieldValue, FieldsSpec, types::FieldSegment},
    context::ResolvedContext,
};

enum StructNode {
    Leaf {
        rust_type: TokenStream,
        value: FieldValue,
    },
    Interior(StructTree),
}

pub struct StructTree(BTreeMap<String, StructNode>);

impl StructTree {
    fn new() -> Self {
        StructTree(BTreeMap::new())
    }

    fn as_inner(&self) -> &BTreeMap<String, StructNode> {
        &self.0
    }

    fn as_inner_mut(&mut self) -> &mut BTreeMap<String, StructNode> {
        &mut self.0
    }

    fn insert(&mut self, path: &[String], spec: FieldSpec) {
        match path {
            [] => {}
            [leaf] => {
                self.as_inner_mut().insert(
                    leaf.clone(),
                    StructNode::Leaf {
                        rust_type: (spec.rust_type)(),
                        value: spec.value,
                    },
                );
            }
            [head, rest @ ..] => {
                if let StructNode::Interior(subtree) = self
                    .as_inner_mut()
                    .entry(head.clone())
                    .or_insert_with(|| StructNode::Interior(StructTree::new()))
                {
                    subtree.insert(rest, spec);
                }
            }
        }
    }

    fn generate_structs(&self, name_parts: &[String], out: &mut Vec<TokenStream>) {
        let struct_ident = Ident::new(&name_parts.join(""), Span::call_site());

        let fields = self
            .as_inner()
            .iter()
            .map(|(field_name, node)| {
                let field_ident = Ident::new(field_name, Span::call_site());

                match node {
                    StructNode::Leaf { rust_type, value } => {
                        let ty = if value.is_secret() {
                            quote! { agentc_config::secret::Secret<#rust_type> }
                        } else {
                            rust_type.clone()
                        };
                        let attribute = value.attribute();

                        quote! { #attribute pub #field_ident: #ty, }
                    }
                    StructNode::Interior(_) => {
                        let child_ident = Ident::new(
                            &name_parts
                                .iter()
                                .cloned()
                                .chain(once(field_name.to_case(Case::Pascal)))
                                .collect::<String>(),
                            Span::call_site(),
                        );

                        quote! { #[config(nested)] pub #field_ident: #child_ident, }
                    }
                }
            })
            .collect::<Vec<_>>();

        out.push(quote! {
            #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, ConfigSchema)]
            #[serde(default)]
            pub struct #struct_ident {
                #(#fields)*
            }
        });

        for (field_name, node) in self.as_inner() {
            if let StructNode::Interior(subtree) = node {
                subtree.generate_structs(
                    &name_parts
                        .iter()
                        .cloned()
                        .chain(once(field_name.to_case(Case::Pascal)))
                        .collect::<Vec<_>>(),
                    out,
                );
            }
        }
    }

    fn generate_config_fields(&self, name_prefix: &str) -> Vec<TokenStream> {
        self.as_inner()
            .iter()
            .map(|(field_name, node)| {
                let field_ident = Ident::new(field_name, Span::call_site());

                match node {
                    StructNode::Leaf { rust_type, value } => {
                        let ty = if value.is_secret() {
                            quote! { agentc_config::secret::Secret<#rust_type> }
                        } else {
                            rust_type.clone()
                        };
                        let attribute = value.attribute();

                        quote! { #attribute pub #field_ident: #ty, }
                    }
                    StructNode::Interior(_) => {
                        let child_ident = Ident::new(
                            &format!("{}{}", name_prefix, field_name.to_case(Case::Pascal)),
                            Span::call_site(),
                        );

                        quote! { #[config(nested)] pub #field_ident: #child_ident, }
                    }
                }
            })
            .collect()
    }

    fn generate_loader_calls(&self, path_segments: &[FieldSegment], tokens: &mut FieldTokens) {
        for (field_name, node) in self.as_inner() {
            let mut current_path = path_segments.to_vec();
            current_path.push(field_name.clone().into());

            match node {
                StructNode::Leaf { value, .. } => {
                    tokens.extend(value.inner_tokens(&current_path));
                }
                StructNode::Interior(subtree) => {
                    subtree.generate_loader_calls(&current_path, tokens);
                }
            }
        }
    }
}

impl FromIterator<FieldSpec> for StructTree {
    fn from_iter<I: IntoIterator<Item = FieldSpec>>(iter: I) -> Self {
        let mut tree = StructTree::new();

        for spec in iter {
            tree.insert(&spec.path.clone(), spec);
        }

        tree
    }
}

pub struct ConfigCodeGen {
    pub fields: FieldsSpec,
}

impl CodeGen<ResolvedContext> for ConfigCodeGen {
    fn generate_files(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError> {
        let config_mods = registry
            .get("config::mods")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let extra_use = registry
            .get("config::use")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let extra_fields = registry
            .get("config::fields")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let extra_impls = registry
            .get("config::impls")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let extra_loader = registry
            .get("config::loader")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let extra_mapper = registry
            .get("config::mapper")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let section_use = registry
            .get("config::sections::use")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let section_types = registry
            .get("config::sections::types")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let section_fields = registry
            .get("config::sections::fields")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let section_loader = registry
            .get("config::sections::loader")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let section_mapper = registry
            .get("config::sections::mapper")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let tree = self
            .fields
            .iter()
            .cloned()
            .collect::<StructTree>();

        let mut generated_structs = Vec::new();

        for (field_name, node) in tree.as_inner() {
            if let StructNode::Interior(subtree) = node {
                subtree.generate_structs(
                    &[format!("Config{}", field_name.to_case(Case::Pascal))],
                    &mut generated_structs,
                );
            }
        }

        let config_generated_fields = tree.generate_config_fields("Config");
        let mut tokens = FieldTokens::default();

        tree.generate_loader_calls(&[], &mut tokens);

        let loader = tokens.loader();
        let aliases = tokens.mapper();

        let mapper = quote! {
            .mapper(
                PrefixMapper::new("AGENT", "__")
                    #(#aliases)*
                    #extra_mapper
                    #section_mapper
            )
        };

        let source = quote! {
            #config_mods

            use std::collections::HashMap;
            use serde::{Serialize, Deserialize};
            use anyhow::Result;

            use agentc_config::{
                traits::{ConfigSchema, OsEnvSource, PrefixMapper},
                macros::{ConfigSchema, path},
            };

            #extra_use
            #section_use

            #section_types

            #(#generated_structs)*

            #[derive(Debug, Clone, Serialize, Deserialize, Default, ConfigSchema)]
            #[serde(default)]
            pub struct Config {
                #section_fields
                #(#config_generated_fields)*
                #extra_fields
            }

            impl Config {
                pub async fn load() -> Result<Self> {
                    Ok(
                        Self::schema()
                            .source(OsEnvSource)
                            #(#loader)*
                            #extra_loader
                            #section_loader
                            #mapper
                            .build()
                            .await?
                            .try_deserialize::<Self>()?
                    )
                }
            }

            #extra_impls
        };

        Ok(vec![("src/config/mod.rs".into(), source)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RuntimeValue;
    use serde_json::json;

    fn context() -> GenerationContext<ResolvedContext> {
        GenerationContext::new(
            serde_json::from_value(json!({
                "slug": "assistant",
                "agent_name": "assistant",
                "runtime": { "default_tenant_id": "default" },
                "providers": {},
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

    fn rendered() -> String {
        ConfigCodeGen { fields: FieldsSpec::new(vec![]) }
            .generate_files(&context(), &ExtensionRegistry::empty())
            .unwrap()
            .into_iter()
            .find(|(path, _)| path == &PathBuf::from("src/config/mod.rs"))
            .expect("config file should be generated")
            .1
            .to_string()
    }

    #[test]
    fn nested_field_references_a_pascal_cased_struct_name() {
        // convert_case splits on digit boundaries, so the "gpt_4o" segment becomes
        // "Gpt4O". The field name itself keeps the raw slug, only the generated
        // struct type is PascalCased.
        let tree = StructTree::from_iter(vec![FieldSpec::new(
            &["gpt_4o", "temperature"],
            &RuntimeValue::constant(0.5f64),
        )]);

        let rendered = tree
            .generate_config_fields("Config")
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(" ")
            .replace(' ', "");

        assert!(rendered.contains("gpt_4o:ConfigGpt4O"));
    }

    #[test]
    fn config_has_no_database_section_when_no_sections_are_contributed() {
        let rendered = rendered();

        assert!(!rendered.contains("struct ConfigDatabase"));
        assert!(!rendered.contains("struct ConfigTaskQueue"));
        assert!(!rendered.contains("enum ConfigPubSub"));
    }

    #[test]
    fn a_leaf_declares_itself_by_attribute_and_an_interior_node_is_nested() {
        let rendered = ConfigCodeGen {
            fields: FieldsSpec::new(vec![
                FieldSpec::new(
                    &["server", "port"],
                    &RuntimeValue::default_runtime("PORT", 8080u16),
                ),
                FieldSpec::new(
                    &["agent", "model", "name"],
                    &RuntimeValue::constant("claude".to_string()),
                ),
            ]),
        }
        .generate_files(&context(), &ExtensionRegistry::empty())
        .unwrap()
        .remove(0)
        .1
        .to_string();

        assert!(
            rendered.contains(
                &quote! {
                    pub struct ConfigServer {
                        #[config(default = serde_json::json!(8080))]
                        pub port: u16,
                    }
                }
                .to_string()
            )
        );
        assert!(
            rendered.contains(
                &quote! {
                    pub struct ConfigAgentModel {
                        #[config(constant = serde_json::json!("claude"))]
                        pub name: String,
                    }
                }
                .to_string()
            )
        );
        assert!(
            rendered.contains(
                &quote! {
                    #[config(nested)]
                    pub agent: ConfigAgent,
                    #[config(nested)]
                    pub server: ConfigServer,
                }
                .to_string()
            )
        );
        assert!(rendered.contains(&quote! { .alias(path!["server", "port"], "PORT") }.to_string()));
        assert!(!rendered.contains(". default (path !"));
        assert!(!rendered.contains(". constant (path !"));
    }

    #[test]
    fn the_config_derives_its_schema_and_loads_from_it() {
        let rendered = rendered();

        assert!(
            rendered.contains(
                &quote! { #[derive(Debug, Clone, Serialize, Deserialize, Default, ConfigSchema)] }
                    .to_string()
            )
        );
        assert!(rendered.contains(&quote! { Self::schema().source(OsEnvSource) }.to_string()));
        assert!(rendered.contains(&quote! { macros::{ConfigSchema, path} }.to_string()));
    }
}
