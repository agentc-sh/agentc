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
        CargoDependencyContribution, CargoPatchContribution, RuntimeDependencyContribution,
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
            "cargo::dependencies" => {
                Ok(ErasedContributionValue::new(CargoDependencyContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-agent-react")
                        .default_features(false),
                )))
            }
            "cargo::patches" => Ok(ErasedContributionValue::new(CargoPatchContribution::runtime(
                RuntimeDependencyContribution::new("agentc-agent-react"),
            ))),
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
            "cargo::dependencies" => {
                Ok(ErasedContributionValue::new(CargoDependencyContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-agent-react")
                        .default_features(false)
                        .feature(self.feature),
                )))
            }
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
