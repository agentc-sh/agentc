// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use std::path::PathBuf;

use agentc_compiler::generator::{
    blocks::codegen::CodeGen, context::GenerationContext, errors::GeneratorError,
    extension::ExtensionRegistry,
};

use crate::{context::ResolvedContext, fields::FieldsSpec};

pub struct ServerCodeGen {
    pub fields: FieldsSpec,
}

impl CodeGen<ResolvedContext> for ServerCodeGen {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<TokenStream, GeneratorError> {
        match point {
            "main::modules" => Ok(quote! {
                mod server;
            }),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }

    fn generate_files(
        &self,
        ctx: &GenerationContext<ResolvedContext>,
        registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, TokenStream)>, GeneratorError> {
        let agent_name = &ctx.agent_name;
        let agent_version = &ctx.agent.version;
        let agent_description = &ctx
            .agent
            .description
            .as_ref()
            .map(|d| quote! { description = #d, })
            .unwrap_or_else(|| quote! {});

        let host_field = self
            .fields
            .config_accessor(&["server", "host"])
            .expect("server.host field is required");
        let port_field = self
            .fields
            .config_accessor(&["server", "port"])
            .expect("server.port field is required");
        let default_tenant_id_field = self
            .fields
            .config_accessor(&["default_tenant_id"])
            .expect("default_tenant_id field is required");

        // Extra routers injected by protocol blocks via extension point
        let extra_routers = registry
            .get("server::routers")
            .and_then(|s| s.parse::<TokenStream>().ok());

        // Extra use statements from protocol blocks
        let extra_use = registry
            .get("server::use")
            .and_then(|s| s.parse::<TokenStream>().ok());

        let server_source = quote! {
            use std::sync::Arc;
            use anyhow::Result;
            use utoipa::OpenApi;

            use agentc_http::{
                server::HttpServer,
                state::ApiState,
            };
            use agentc_agent_react::{
                service::ApplicationService,
                api::router::v1,
            };

            #extra_use

            use crate::config::Config;

            #[derive(OpenApi)]
            #[openapi(
                info(
                    title = #agent_name,
                    version = #agent_version,
                    #agent_description
                ),
            )]
            struct ApiDoc;

            pub fn build(
                service: Arc<ApplicationService>,
                config: &Config,
            ) -> Result<HttpServer> {
                let state = ApiState::new_arc(
                    service,
                    #default_tenant_id_field.clone(),
                );

                let mut builder = HttpServer::builder()
                    .with_openapi(ApiDoc::openapi())
                    .with_router(v1::router(state.clone()));

                #extra_routers

                builder
                    .with_host(#host_field.clone())
                    .with_port(#port_field)
                    .build()
            }
        };

        Ok(vec![("src/server.rs".into(), server_source)])
    }
}
