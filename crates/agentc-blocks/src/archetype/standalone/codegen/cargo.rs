// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

use agentc_compiler::generator::{
    blocks::template::TemplateFragment,
    context::GenerationContext,
    errors::GeneratorError,
    extension::{ErasedContributionValue, ExtensionPoint, ExtensionRegistry},
};

use crate::{
    context::ResolvedContext,
    contributions::dependency::{
        CargoDependencies, CargoDependencyContribution, CargoPatches, CargoPatchContribution,
        RuntimeDependencyContribution, ExternalDependencyContribution,
    },
};

pub struct A2aClientCargoFragment;

impl TemplateFragment<ResolvedContext> for A2aClientCargoFragment {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "cargo::dependencies" => Ok(ErasedContributionValue::new(
                CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-protocol-a2a")
                        .default_features(false)
                        .feature("client"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            "cargo::patches" => Ok(ErasedContributionValue::new(
                CargoPatches::from_entries([CargoPatchContribution::runtime(
                    RuntimeDependencyContribution::new("agentc-protocol-a2a"),
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

pub struct HttpClientCargoFragment;

impl TemplateFragment<ResolvedContext> for HttpClientCargoFragment {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
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

    fn generate_files(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        _registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, String)>, GeneratorError> {
        Ok(vec![])
    }
}

pub struct HttpServerCargoFragment;

impl TemplateFragment<ResolvedContext> for HttpServerCargoFragment {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "cargo::dependencies" => Ok(ErasedContributionValue::new(
                CargoDependencies::from_entries([
                    CargoDependencyContribution::runtime(
                        RuntimeDependencyContribution::new("agentc-http")
                            .default_features(false)
                            .feature("server"),
                    ),
                    CargoDependencyContribution::external(
                        ExternalDependencyContribution::new("jobq")
                            .git("https://github.com/wizrds/jobq-rs.git")
                            .version("0.3.1"),
                    ),
                    CargoDependencyContribution::external(
                        ExternalDependencyContribution::new("subway")
                            .git("https://github.com/wizrds/subway-rs.git")
                            .version("0.1.0")
                            .feature("redis"),
                    ),
                    CargoDependencyContribution::external(
                        ExternalDependencyContribution::new("utoipa").version("5.4"),
                    ),
                    CargoDependencyContribution::external(
                        ExternalDependencyContribution::new("utoipa-axum").version("0.2"),
                    ),
                ])
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

#[derive(Debug, Clone)]
pub struct CargoDependenciesExtensionPoint {
    name: &'static str,
    runtime_version: &'static str,
}

impl CargoDependenciesExtensionPoint {
    pub fn new(name: &'static str, runtime_version: &'static str) -> Self {
        Self { name, runtime_version }
    }

    fn render_runtime_dependency(&self, dependency: RuntimeDependencyContribution) -> String {
        let mut fields = vec![format!("version = \"{}\"", self.runtime_version)];

        if let Some(default_features) = dependency.default_features {
            fields.push(format!("default-features = {default_features}"));
        }

        if !dependency.features.is_empty() {
            fields.push(format!(
                "features = [{}]",
                dependency
                    .features
                    .into_iter()
                    .map(|feature| format!("\"{feature}\""))
                    .collect::<Vec<_>>()
                    .join(", "),
            ));
        }

        format!("{} = {{ {} }}", dependency.name, fields.join(", "))
    }

    fn render_external_dependency(&self, dependency: ExternalDependencyContribution) -> String {
        let mut fields = Vec::new();

        if let Some(path) = dependency.path {
            fields.push(format!("path = \"{path}\""));
        }

        if let Some(git) = dependency.git {
            fields.push(format!("git = \"{git}\""));
        }

        if let Some(version) = dependency.version {
            fields.push(format!("version = \"{version}\""));
        }

        if let Some(branch) = dependency.branch {
            fields.push(format!("branch = \"{branch}\""));
        }

        if let Some(tag) = dependency.tag {
            fields.push(format!("tag = \"{tag}\""));
        }

        if let Some(rev) = dependency.rev {
            fields.push(format!("rev = \"{rev}\""));
        }

        if let Some(default_features) = dependency.default_features {
            fields.push(format!("default-features = {default_features}"));
        }

        if !dependency.features.is_empty() {
            fields.push(format!(
                "features = [{}]",
                dependency
                    .features
                    .into_iter()
                    .map(|feature| format!("\"{feature}\""))
                    .collect::<Vec<_>>()
                    .join(", "),
            ));
        }

        format!("{} = {{ {} }}", dependency.name, fields.join(", "))
    }

    fn render(&self, dependency: CargoDependencyContribution) -> String {
        match dependency {
            CargoDependencyContribution::Runtime(dependency) => {
                self.render_runtime_dependency(dependency)
            }
            CargoDependencyContribution::External(dependency) => {
                self.render_external_dependency(dependency)
            }
        }
    }
}

impl ExtensionPoint for CargoDependenciesExtensionPoint {
    type Contribution = CargoDependencies;

    fn name(&self) -> &str {
        self.name
    }

    fn reduce(&self, contributions: Vec<Self::Contribution>) -> Result<String, GeneratorError> {
        Ok(
            CargoDependencies::merge_all(contributions)
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?
                .into_values()
                .map(|dependency| self.render(dependency))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

#[derive(Debug, Clone)]
pub struct CargoPatchesExtensionPoint {
    name: &'static str,
}

impl CargoPatchesExtensionPoint {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }

    fn render_runtime_patch(&self, dependency: RuntimeDependencyContribution) -> String {
        format!("{} = {{ path = \"../runtime/{}\" }}", dependency.name, dependency.name,)
    }

}

impl ExtensionPoint for CargoPatchesExtensionPoint {
    type Contribution = CargoPatches;

    fn name(&self) -> &str {
        self.name
    }

    fn reduce(&self, contributions: Vec<Self::Contribution>) -> Result<String, GeneratorError> {
        Ok(
            CargoPatches::merge_all(contributions)
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?
                .into_values()
                .map(|patch| self.render_runtime_patch(patch.dependency))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    use crate::context::ResolvedContext;

    fn dependencies() -> CargoDependenciesExtensionPoint {
        CargoDependenciesExtensionPoint::new("cargo::dependencies", "0.2.1")
    }

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
    fn runtime_dependencies_with_same_name_merge_features() {
        assert_eq!(
            ExtensionPoint::reduce(
                &dependencies(),
                vec![
                    CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                        RuntimeDependencyContribution::new("agentc-protocol-a2a")
                            .default_features(false)
                            .feature("server"),
                    )])
                    .unwrap(),
                    CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                        RuntimeDependencyContribution::new("agentc-protocol-a2a")
                            .default_features(false)
                            .feature("client"),
                    )])
                    .unwrap(),
                ],
            )
            .unwrap(),
            "agentc-protocol-a2a = { version = \"0.2.1\", default-features = false, features = [\"client\", \"server\"] }",
        );
    }

    #[test]
    fn conflicting_runtime_default_features_error() {
        assert!(
            ExtensionPoint::reduce(
                &dependencies(),
                vec![
                    CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                        RuntimeDependencyContribution::new("dep").default_features(true),
                    )])
                    .unwrap(),
                    CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                        RuntimeDependencyContribution::new("dep").default_features(false),
                    )])
                    .unwrap(),
                ],
            )
            .is_err()
        );
    }

    #[test]
    fn runtime_dependencies_render_in_package_name_order() {
        assert_eq!(
            ExtensionPoint::reduce(
                &dependencies(),
                vec![
                    CargoDependencies::from_entries([
                        CargoDependencyContribution::runtime(RuntimeDependencyContribution::new(
                            "zzz",
                        )),
                        CargoDependencyContribution::runtime(RuntimeDependencyContribution::new(
                            "aaa",
                        )),
                    ])
                    .unwrap(),
                ],
            )
            .unwrap(),
            "aaa = { version = \"0.2.1\" }\nzzz = { version = \"0.2.1\" }",
        );
    }

    #[test]
    fn runtime_dependency_without_features_renders_version_only() {
        assert_eq!(
            ExtensionPoint::reduce(
                &dependencies(),
                vec![
                    CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                        RuntimeDependencyContribution::new("dep"),
                    )])
                    .unwrap(),
                ],
            )
            .unwrap(),
            "dep = { version = \"0.2.1\" }",
        );
    }

    #[test]
    fn runtime_dependency_with_disabled_default_features_renders_flag() {
        assert_eq!(
            ExtensionPoint::reduce(
                &dependencies(),
                vec![
                    CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                        RuntimeDependencyContribution::new("dep").default_features(false),
                    )])
                    .unwrap(),
                ],
            )
            .unwrap(),
            "dep = { version = \"0.2.1\", default-features = false }",
        );
    }

    #[test]
    fn runtime_patches_with_same_name_merge_into_one_line() {
        assert_eq!(
            ExtensionPoint::reduce(
                &CargoPatchesExtensionPoint::new("cargo::patches"),
                vec![
                    CargoPatches::from_entries([CargoPatchContribution::runtime(
                        RuntimeDependencyContribution::new("agentc-protocol-a2a").feature("server"),
                    )])
                    .unwrap(),
                    CargoPatches::from_entries([CargoPatchContribution::runtime(
                        RuntimeDependencyContribution::new("agentc-protocol-a2a").feature("client"),
                    )])
                    .unwrap(),
                ],
            )
            .unwrap(),
            "agentc-protocol-a2a = { path = \"../runtime/agentc-protocol-a2a\" }",
        );
    }

    #[test]
    fn one_fragment_can_declare_several_dependencies() {
        assert_eq!(
            ExtensionPoint::reduce(
                &dependencies(),
                vec![
                    HttpServerCargoFragment
                        .generate_contribution(&context(), "cargo::dependencies")
                        .unwrap()
                        .downcast::<CargoDependencies>()
                        .unwrap(),
                ],
            )
            .unwrap(),
            concat!(
                "agentc-http = { version = \"0.2.1\", default-features = false, features = [\"server\"] }\n",
                "jobq = { git = \"https://github.com/wizrds/jobq-rs.git\", version = \"0.3.1\" }\n",
                "subway = { git = \"https://github.com/wizrds/subway-rs.git\", version = \"0.1.0\", features = [\"redis\"] }\n",
                "utoipa = { version = \"5.4\" }\n",
                "utoipa-axum = { version = \"0.2\" }",
            ),
        );
    }

    #[test]
    fn dependencies_merge_the_same_way_within_and_across_fragments() {
        let within = ExtensionPoint::reduce(
            &dependencies(),
            vec![
                CargoDependencies::from_entries([
                    CargoDependencyContribution::runtime(
                        RuntimeDependencyContribution::new("dep").feature("server"),
                    ),
                    CargoDependencyContribution::runtime(
                        RuntimeDependencyContribution::new("dep").feature("client"),
                    ),
                ])
                .unwrap(),
            ],
        )
        .unwrap();
        let across = ExtensionPoint::reduce(
            &dependencies(),
            vec![
                CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                    RuntimeDependencyContribution::new("dep").feature("server"),
                )])
                .unwrap(),
                CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                    RuntimeDependencyContribution::new("dep").feature("client"),
                )])
                .unwrap(),
            ],
        )
        .unwrap();

        assert_eq!(within, across);
        assert_eq!(within, "dep = { version = \"0.2.1\", features = [\"client\", \"server\"] }");
    }

    #[test]
    fn external_dependencies_render_after_merging_features() {
        assert_eq!(
            ExtensionPoint::reduce(
                &dependencies(),
                vec![
                    CargoDependencies::from_entries([CargoDependencyContribution::external(
                        ExternalDependencyContribution::new("subway")
                            .git("https://github.com/wizrds/subway-rs.git")
                            .version("0.1.0")
                            .feature("redis"),
                    )])
                    .unwrap(),
                    CargoDependencies::from_entries([CargoDependencyContribution::external(
                        ExternalDependencyContribution::new("subway").feature("tls"),
                    )])
                    .unwrap(),
                ],
            )
            .unwrap(),
            "subway = { git = \"https://github.com/wizrds/subway-rs.git\", version = \"0.1.0\", features = [\"redis\", \"tls\"] }",
        );
    }

    #[test]
    fn a_package_declared_as_both_runtime_and_external_is_an_error() {
        assert!(
            ExtensionPoint::reduce(
                &dependencies(),
                vec![
                    CargoDependencies::from_entries([CargoDependencyContribution::runtime(
                        RuntimeDependencyContribution::new("dep"),
                    )])
                    .unwrap(),
                    CargoDependencies::from_entries([CargoDependencyContribution::external(
                        ExternalDependencyContribution::new("dep").version("1"),
                    )])
                    .unwrap(),
                ],
            )
            .is_err()
        );
    }

    #[test]
    fn a2a_client_fragment_contributes_client_runtime_dependency() {
        let dependencies = A2aClientCargoFragment
            .generate_contribution(&context(), "cargo::dependencies")
            .unwrap()
            .downcast::<CargoDependencies>()
            .unwrap();

        assert_eq!(dependencies.len(), 1);
        assert!(matches!(
            dependencies
                .get(&"agentc-protocol-a2a")
                .unwrap(),
            CargoDependencyContribution::Runtime(dependency)
                if dependency.default_features == Some(false)
                    && dependency.features.len() == 1
                    && dependency.features.contains("client")
        ));
    }

    #[test]
    fn a2a_client_fragment_contributes_runtime_patch() {
        let patches = A2aClientCargoFragment
            .generate_contribution(&context(), "cargo::patches")
            .unwrap()
            .downcast::<CargoPatches>()
            .unwrap();

        assert_eq!(patches.len(), 1);
        assert!(
            patches
                .get(&"agentc-protocol-a2a")
                .is_some()
        );
    }
}
