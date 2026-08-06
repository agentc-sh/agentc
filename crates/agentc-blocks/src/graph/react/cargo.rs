// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

use agentc_compiler::generator::{
    blocks::template::TemplateFragment,
    context::GenerationContext,
    errors::GeneratorError,
    extension::{ErasedContributionValue, ExtensionRegistry},
};

use crate::{
    context::ResolvedContext,
    contributions::dependency::{
        CargoDependencies, CargoDependencyContribution, CargoPatches, CargoPatchContribution,
        RuntimeDependencyContribution,
    },
};

pub struct ReActCargoFragment;

impl TemplateFragment<ResolvedContext> for ReActCargoFragment {
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

    fn generate_files(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        _registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, String)>, GeneratorError> {
        Ok(vec![])
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

impl TemplateFragment<ResolvedContext> for ReActFeatureCargoFragment {
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

    fn generate_files(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        _registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, String)>, GeneratorError> {
        Ok(vec![])
    }
}
