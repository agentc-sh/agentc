// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_compiler::generator::{
    blocks::fragment::Fragment, context::GenerationContext, errors::GeneratorError,
    extension::ErasedContributionValue,
};

use crate::{
    context::ResolvedContext,
    contributions::dependency::{
        CargoDependencies, CargoDependencyContribution, CargoPatchContribution, CargoPatches,
        ExternalDependencyContribution, RuntimeDependencyContribution,
    },
};

pub struct ReActCargoFragment;

impl Fragment<ResolvedContext> for ReActCargoFragment {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "cargo::dependencies" => Ok(ErasedContributionValue::new(
                CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-agent-react")
                        .default_features(false),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            "cargo::patches" => Ok(ErasedContributionValue::new(
                CargoPatches::from_entries([CargoPatchContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-agent-react"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }
}

pub struct ReActDatabaseCargoFragment;

impl Fragment<ResolvedContext> for ReActDatabaseCargoFragment {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "cargo::dependencies" => Ok(ErasedContributionValue::new(
                CargoDependencies::from_entries([
                    CargoDependencyContribution::runtime(RuntimeDependencyContribution::new(
                        "agentc-domain-sql",
                    )),
                    CargoDependencyContribution::external(
                        ExternalDependencyContribution::new("sea-orm-migration").version("1"),
                    ),
                ])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            "cargo::patches" => Ok(ErasedContributionValue::new(
                CargoPatches::from_entries([CargoPatchContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-domain-sql"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }
}

/// Enables one cargo feature on the react runtime dependency.
pub struct ReActFeatureCargoFragment {
    feature: &'static str,
}

impl ReActFeatureCargoFragment {
    /// Creates a fragment enabling the named feature.
    pub fn new(feature: &'static str) -> Self {
        Self { feature }
    }
}

impl Fragment<ResolvedContext> for ReActFeatureCargoFragment {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "cargo::dependencies" => Ok(ErasedContributionValue::new(
                CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-agent-react")
                        .default_features(false)
                        .feature(self.feature),
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
    fn database_fragment_contributes_sql_dependencies() {
        let dependencies = ReActDatabaseCargoFragment
            .generate_contribution(&context(), "cargo::dependencies")
            .unwrap()
            .downcast::<CargoDependencies>()
            .unwrap();

        assert!(dependencies
            .get(&"agentc-domain-sql")
            .is_some());
        assert!(dependencies
            .get(&"sea-orm-migration")
            .is_some());
    }
}
