// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use agentc_compiler::generator::{
    errors::GeneratorError,
    extension::ExtensionPoint,
};

use crate::{
    contributions::dependency::RuntimeDependencyContribution,
    errors::BlocksError,
};

#[derive(Debug, Clone)]
pub enum CargoDependencyContribution {
    Raw(String),
    Runtime(RuntimeDependencyContribution),
}

impl CargoDependencyContribution {
    pub fn raw(value: impl Into<String>) -> Self {
        Self::Raw(value.into())
    }

    pub fn runtime(dependency: impl Into<RuntimeDependencyContribution>) -> Self {
        Self::Runtime(dependency.into())
    }
}

impl From<String> for CargoDependencyContribution {
    fn from(value: String) -> Self {
        Self::Raw(value)
    }
}

impl From<&str> for CargoDependencyContribution {
    fn from(value: &str) -> Self {
        Self::Raw(value.to_string())
    }
}

#[derive(Debug, Clone)]
pub enum CargoPatchContribution {
    Raw(String),
    Runtime(RuntimeDependencyContribution),
}

impl CargoPatchContribution {
    pub fn raw(value: impl Into<String>) -> Self {
        Self::Raw(value.into())
    }

    pub fn runtime(dependency: impl Into<RuntimeDependencyContribution>) -> Self {
        Self::Runtime(dependency.into())
    }
}

impl From<String> for CargoPatchContribution {
    fn from(value: String) -> Self {
        Self::Raw(value)
    }
}

impl From<&str> for CargoPatchContribution {
    fn from(value: &str) -> Self {
        Self::Raw(value.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct CargoDependenciesExtensionPoint {
    name: &'static str,
    runtime_version: &'static str,
}

impl CargoDependenciesExtensionPoint {
    pub fn new(name: &'static str, runtime_version: &'static str) -> Self {
        Self {
            name,
            runtime_version,
        }
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

    fn merge_runtime_dependency(
        dependencies: &mut BTreeMap<&'static str, RuntimeDependencyContribution>,
        dependency: RuntimeDependencyContribution,
    ) -> Result<(), BlocksError> {
        if let Some(existing) = dependencies.get_mut(dependency.name) {
            existing.merge(dependency)?;
        } else {
            dependencies.insert(dependency.name, dependency);
        }

        Ok(())
    }
}

impl ExtensionPoint for CargoDependenciesExtensionPoint {
    type Contribution = CargoDependencyContribution;

    fn name(&self) -> &str {
        self.name
    }

    fn reduce(
        &self,
        contributions: Vec<Self::Contribution>,
    ) -> Result<String, GeneratorError> {
        let mut raw = Vec::new();
        let mut runtime = BTreeMap::new();

        for contribution in contributions {
            match contribution {
                CargoDependencyContribution::Raw(value) => raw.push(value),
                CargoDependencyContribution::Runtime(dependency) => {
                    Self::merge_runtime_dependency(&mut runtime, dependency)
                        .map_err(|error| GeneratorError::unexpected(error.to_string()))?;
                }
            }
        }

        Ok(
            raw.into_iter()
                .chain(
                    runtime
                        .into_values()
                        .map(|dependency| self.render_runtime_dependency(dependency)),
                )
                .collect::<Vec<_>>()
                .join("\n"),
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
        format!(
            "{} = {{ path = \"../runtime/{}\" }}",
            dependency.name,
            dependency.name,
        )
    }

    fn merge_runtime_patch(
        patches: &mut BTreeMap<&'static str, RuntimeDependencyContribution>,
        dependency: RuntimeDependencyContribution,
    ) -> Result<(), BlocksError> {
        if let Some(existing) = patches.get_mut(dependency.name) {
            existing.merge(dependency)?;
        } else {
            patches.insert(dependency.name, dependency);
        }

        Ok(())
    }
}

impl ExtensionPoint for CargoPatchesExtensionPoint {
    type Contribution = CargoPatchContribution;

    fn name(&self) -> &str {
        self.name
    }

    fn reduce(
        &self,
        contributions: Vec<Self::Contribution>,
    ) -> Result<String, GeneratorError> {
        let mut raw = Vec::new();
        let mut runtime = BTreeMap::new();

        for contribution in contributions {
            match contribution {
                CargoPatchContribution::Raw(value) => raw.push(value),
                CargoPatchContribution::Runtime(dependency) => {
                    Self::merge_runtime_patch(&mut runtime, dependency)
                        .map_err(|error| GeneratorError::unexpected(error.to_string()))?;
                }
            }
        }

        Ok(
            raw.into_iter()
                .chain(
                    runtime
                        .into_values()
                        .map(|dependency| self.render_runtime_patch(dependency)),
                )
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dependencies() -> CargoDependenciesExtensionPoint {
        CargoDependenciesExtensionPoint::new("cargo::dependencies", "0.2.1")
    }

    #[test]
    fn raw_dependencies_render_in_input_order() {
        assert_eq!(
            ExtensionPoint::reduce(
                &dependencies(),
                vec![
                    CargoDependencyContribution::raw("b = { version = \"1\" }"),
                    CargoDependencyContribution::raw("a = { version = \"1\" }"),
                ],
            )
            .unwrap(),
            "b = { version = \"1\" }\na = { version = \"1\" }",
        );
    }

    #[test]
    fn runtime_dependencies_with_same_name_merge_features() {
        assert_eq!(
            ExtensionPoint::reduce(
                &dependencies(),
                vec![
                    CargoDependencyContribution::runtime(
                        RuntimeDependencyContribution::new("agentc-protocol-a2a")
                            .default_features(false)
                            .feature("server"),
                    ),
                    CargoDependencyContribution::runtime(
                        RuntimeDependencyContribution::new("agentc-protocol-a2a")
                            .default_features(false)
                            .feature("client"),
                    ),
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
                    CargoDependencyContribution::runtime(
                        RuntimeDependencyContribution::new("dep").default_features(true),
                    ),
                    CargoDependencyContribution::runtime(
                        RuntimeDependencyContribution::new("dep").default_features(false),
                    ),
                ],
            )
            .is_err()
        );
    }

    #[test]
    fn raw_and_runtime_dependencies_render_into_one_string() {
        assert_eq!(
            ExtensionPoint::reduce(
                &dependencies(),
                vec![
                    CargoDependencyContribution::raw("serde = { version = \"1\" }"),
                    CargoDependencyContribution::runtime(RuntimeDependencyContribution::new(
                        "tokio",
                    )),
                ],
            )
            .unwrap(),
            "serde = { version = \"1\" }\ntokio = { version = \"0.2.1\" }",
        );
    }

    #[test]
    fn runtime_dependencies_render_in_package_name_order() {
        assert_eq!(
            ExtensionPoint::reduce(
                &dependencies(),
                vec![
                    CargoDependencyContribution::runtime(RuntimeDependencyContribution::new("zzz")),
                    CargoDependencyContribution::runtime(RuntimeDependencyContribution::new("aaa")),
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
                vec![CargoDependencyContribution::runtime(
                    RuntimeDependencyContribution::new("dep"),
                )],
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
                vec![CargoDependencyContribution::runtime(
                    RuntimeDependencyContribution::new("dep").default_features(false),
                )],
            )
            .unwrap(),
            "dep = { version = \"0.2.1\", default-features = false }",
        );
    }

    #[test]
    fn raw_patches_render_in_input_order() {
        assert_eq!(
            ExtensionPoint::reduce(
                &CargoPatchesExtensionPoint::new("cargo::patches"),
                vec![
                    CargoPatchContribution::raw("b = { path = \"../b\" }"),
                    CargoPatchContribution::raw("a = { path = \"../a\" }"),
                ],
            )
            .unwrap(),
            "b = { path = \"../b\" }\na = { path = \"../a\" }",
        );
    }

    #[test]
    fn runtime_patches_with_same_name_merge_into_one_line() {
        assert_eq!(
            ExtensionPoint::reduce(
                &CargoPatchesExtensionPoint::new("cargo::patches"),
                vec![
                    CargoPatchContribution::runtime(
                        RuntimeDependencyContribution::new("agentc-protocol-a2a").feature("server"),
                    ),
                    CargoPatchContribution::runtime(
                        RuntimeDependencyContribution::new("agentc-protocol-a2a").feature("client"),
                    ),
                ],
            )
            .unwrap(),
            "agentc-protocol-a2a = { path = \"../runtime/agentc-protocol-a2a\" }",
        );
    }
}
