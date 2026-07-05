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
    archetype::standalone::fields::{FieldSpec, FieldValue, FieldsSpec},
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
                        let ty = match value {
                            FieldValue::Runtime { secret: true, .. } => {
                                quote! { agentc_config::secret::Secret<#rust_type> }
                            }
                            _ => rust_type.clone(),
                        };

                        quote! { pub #field_ident: #ty, }
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

                        quote! { pub #field_ident: #child_ident, }
                    }
                }
            })
            .collect::<Vec<_>>();

        out.push(quote! {
            #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
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
                        let ty = match value {
                            FieldValue::Runtime { secret: true, .. } => {
                                quote! { agentc_config::secret::Secret<#rust_type> }
                            }
                            _ => rust_type.clone(),
                        };

                        quote! { pub #field_ident: #ty, }
                    }
                    StructNode::Interior(_) => {
                        let child_ident = Ident::new(
                            &format!("{}{}", name_prefix, field_name.to_case(Case::Pascal)),
                            Span::call_site(),
                        );

                        quote! { pub #field_ident: #child_ident, }
                    }
                }
            })
            .collect()
    }

    fn generate_loader_calls(
        &self,
        path_segments: &[String],
        constants: &mut Vec<TokenStream>,
        defaults: &mut Vec<TokenStream>,
        field_mappings: &mut Vec<TokenStream>,
    ) {
        for (field_name, node) in self.as_inner() {
            let mut current_path = path_segments.to_vec();
            current_path.push(field_name.clone());

            match node {
                StructNode::Leaf { value, .. } => match value {
                    FieldValue::Constant { value } => {
                        let json_tokens = value
                            .to_string()
                            .parse::<TokenStream>()
                            .unwrap();

                        constants.push(quote! {
                            .constant(path![#(#current_path),*], serde_json::json!(#json_tokens))
                        });
                    }
                    FieldValue::Runtime { env, default, .. } => {
                        field_mappings.push(quote! {
                            .field(path![#(#current_path),*], #env)
                        });

                        if let Some(default_val) = default {
                            let json_tokens = default_val
                                .to_string()
                                .parse::<TokenStream>()
                                .unwrap();

                            defaults.push(quote! {
                                .default(path![#(#current_path),*], serde_json::json!(#json_tokens))
                            });
                        }
                    }
                },
                StructNode::Interior(subtree) => {
                    subtree.generate_loader_calls(
                        &current_path,
                        constants,
                        defaults,
                        field_mappings,
                    );
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

        let tree = self
            .fields
            .iter()
            .map(|spec| FieldSpec {
                path: spec.path.clone(),
                rust_type: spec.rust_type,
                value: match &spec.value {
                    FieldValue::Constant { value } => FieldValue::Constant { value: value.clone() },
                    FieldValue::Runtime { env, default, secret } => FieldValue::Runtime {
                        env: env.clone(),
                        default: default.clone(),
                        secret: *secret,
                    },
                },
            })
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
        let mut constants = Vec::new();
        let mut defaults = Vec::new();
        let mut field_mappings = Vec::new();

        tree.generate_loader_calls(&[], &mut constants, &mut defaults, &mut field_mappings);

        let mapper = quote! {
            .mapper(
                PrefixMapper::new("AGENT", "__")
                    #(#field_mappings)*
                    #extra_mapper
            )
        };

        let source = quote! {
            use std::collections::HashMap;
            use serde::{Serialize, Deserialize};
            use anyhow::Result;

            use agentc_config::traits::{OsEnvSource, PrefixMapper};
            use agentc_config::macros::path;
            use agentc_database::{
                Database,
                database::DatabaseOptions,
                errors::DatabaseError,
            };

            use crate::migrator::Migrator;

            #extra_use

            #[derive(Debug, Clone, Serialize, Deserialize)]
            #[serde(default)]
            pub struct DatabaseConfig {
                pub primary: String,
                pub replicas: Vec<String>,
                pub options: DatabaseOptions,
            }

            impl DatabaseConfig {
                pub async fn build(&self, run_migrations: bool) -> Result<Database, DatabaseError> {
                    let db = Database::builder()
                        .with_primary(self.primary.clone())
                        .with_replicas(self.replicas.clone())
                        .with_options(self.options.clone())
                        .build()
                        .await?;

                    if run_migrations {
                        db.run_migrations::<Migrator>().await?;
                    }

                    Ok(db)
                }
            }

            impl Default for DatabaseConfig {
                fn default() -> Self {
                    DatabaseConfig {
                        primary: "sqlite://database.db?mode=rwc".to_string(),
                        replicas: vec![],
                        options: DatabaseOptions::default(),
                    }
                }
            }

            #[derive(Debug, Clone, Serialize, Deserialize)]
            #[serde(tag = "type", rename_all = "snake_case")]
            pub enum McpTransportConfig {
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
                    auth_token: Option<String>,
                    #[serde(default)]
                    headers: HashMap<String, String>,
                },
            }

            #[derive(Debug, Clone, Serialize, Deserialize, Default)]
            #[serde(default)]
            pub struct McpConfig {
                pub servers: HashMap<String, McpTransportConfig>,
            }

            #(#generated_structs)*

            #[derive(Debug, Clone, Serialize, Deserialize, Default)]
            #[serde(default)]
            pub struct Config {
                pub database: DatabaseConfig,
                pub mcp: McpConfig,
                #(#config_generated_fields)*
                #extra_fields
            }

            impl Config {
                pub async fn load() -> Result<Self> {
                    Ok(
                        agentc_config::config::Config::builder()
                            .source(OsEnvSource)
                            #(#constants)*
                            #(#defaults)*
                            #extra_loader
                            #mapper
                            .build()
                            .await?
                            .try_deserialize::<Self>()?
                    )
                }
            }

            #extra_impls
        };

        Ok(vec![("src/config.rs".into(), source)])
    }
}
