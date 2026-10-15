// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use crate::{
    context::ResolvedContextBlockDependency,
    contributions::set::{ContributionSet, Mergeable},
    errors::BlocksError,
};

#[derive(Debug, Clone)]
pub enum CargoDependencyContribution {
    Runtime(RuntimeDependencyContribution),
    External(ExternalDependencyContribution),
}

impl CargoDependencyContribution {
    pub fn runtime(dependency: impl Into<RuntimeDependencyContribution>) -> Self {
        Self::Runtime(dependency.into())
    }

    pub fn external(dependency: impl Into<ExternalDependencyContribution>) -> Self {
        Self::External(dependency.into())
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Runtime(dependency) => dependency.name,
            Self::External(dependency) => &dependency.name,
        }
    }
}

impl Mergeable for CargoDependencyContribution {
    type Key = String;

    fn key(&self) -> Self::Key {
        self.name().to_string()
    }

    fn merge(&mut self, other: Self) -> Result<(), BlocksError> {
        match (self, other) {
            (Self::Runtime(left), Self::Runtime(right)) => left.merge(right),
            (Self::External(left), Self::External(right)) => left.merge(right),
            (left, _) => Err(BlocksError::invalid(format!(
                "cannot merge dependency '{}' declared as both a runtime and an external crate",
                left.name(),
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CargoPatchContribution {
    pub dependency: RuntimeDependencyContribution,
}

impl CargoPatchContribution {
    pub fn runtime(dependency: impl Into<RuntimeDependencyContribution>) -> Self {
        Self { dependency: dependency.into() }
    }
}

impl Mergeable for CargoPatchContribution {
    type Key = &'static str;

    fn key(&self) -> Self::Key {
        self.dependency.name
    }

    fn merge(&mut self, other: Self) -> Result<(), BlocksError> {
        self.dependency.merge(other.dependency)
    }
}

pub type CargoDependencies = ContributionSet<CargoDependencyContribution>;
pub type CargoPatches = ContributionSet<CargoPatchContribution>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDependencyContribution {
    pub name: &'static str,
    pub default_features: Option<bool>,
    pub features: BTreeSet<&'static str>,
}

impl RuntimeDependencyContribution {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            default_features: None,
            features: BTreeSet::new(),
        }
    }

    pub fn default_features(mut self, enabled: bool) -> Self {
        self.default_features = Some(enabled);
        self
    }

    pub fn feature(mut self, feature: &'static str) -> Self {
        self.features.insert(feature);
        self
    }

    pub fn merge(&mut self, other: Self) -> Result<(), BlocksError> {
        if self.name != other.name {
            return Err(BlocksError::invalid(format!(
                "cannot merge runtime dependencies '{}' and '{}'",
                self.name, other.name,
            )));
        }

        match (self.default_features, other.default_features) {
            (Some(left), Some(right)) if left != right => {
                return Err(BlocksError::invalid(format!(
                    "conflicting default-features settings for runtime dependency '{}'",
                    self.name,
                )));
            }
            (None, Some(value)) => {
                self.default_features = Some(value);
            }
            _ => {}
        }

        self.features.extend(other.features);

        Ok(())
    }
}

/// A dependency on a crate outside this repository's runtime libraries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalDependencyContribution {
    pub name: String,
    pub version: Option<String>,
    pub git: Option<String>,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub rev: Option<String>,
    pub path: Option<String>,
    pub default_features: Option<bool>,
    pub features: BTreeSet<String>,
}

impl ExternalDependencyContribution {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: None,
            git: None,
            branch: None,
            tag: None,
            rev: None,
            path: None,
            default_features: None,
            features: BTreeSet::new(),
        }
    }

    fn merge_field(
        name: &str,
        field: &'static str,
        left: &mut Option<String>,
        right: Option<String>,
    ) -> Result<(), BlocksError> {
        match (left.as_deref(), right) {
            (Some(left), Some(right)) if left != right => Err(BlocksError::invalid(format!(
                "conflicting {field} values for external dependency '{name}'",
            ))),
            (None, Some(value)) => {
                *left = Some(value);

                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn git(mut self, url: impl Into<String>) -> Self {
        self.git = Some(url.into());
        self
    }

    pub fn branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    pub fn rev(mut self, rev: impl Into<String>) -> Self {
        self.rev = Some(rev.into());
        self
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn default_features(mut self, enabled: bool) -> Self {
        self.default_features = Some(enabled);
        self
    }

    pub fn feature(mut self, feature: impl Into<String>) -> Self {
        self.features.insert(feature.into());
        self
    }

    pub fn merge(&mut self, other: Self) -> Result<(), BlocksError> {
        if self.name != other.name {
            return Err(BlocksError::invalid(format!(
                "cannot merge external dependencies '{}' and '{}'",
                self.name, other.name,
            )));
        }

        Self::merge_field(&self.name, "version", &mut self.version, other.version)?;
        Self::merge_field(&self.name, "git", &mut self.git, other.git)?;
        Self::merge_field(&self.name, "branch", &mut self.branch, other.branch)?;
        Self::merge_field(&self.name, "tag", &mut self.tag, other.tag)?;
        Self::merge_field(&self.name, "rev", &mut self.rev, other.rev)?;
        Self::merge_field(&self.name, "path", &mut self.path, other.path)?;

        match (self.default_features, other.default_features) {
            (Some(left), Some(right)) if left != right => {
                return Err(BlocksError::invalid(format!(
                    "conflicting default-features settings for external dependency '{}'",
                    self.name,
                )));
            }
            (None, Some(value)) => {
                self.default_features = Some(value);
            }
            _ => {}
        }

        self.features.extend(other.features);

        Ok(())
    }
}

impl From<&ResolvedContextBlockDependency> for ExternalDependencyContribution {
    fn from(dependency: &ResolvedContextBlockDependency) -> Self {
        Self {
            name: dependency.name.clone(),
            version: dependency.version.clone(),
            git: dependency.git.clone(),
            branch: dependency.branch.clone(),
            tag: dependency.tag.clone(),
            rev: dependency.rev.clone(),
            path: dependency.path.clone(),
            default_features: dependency.default_features,
            features: dependency.features.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_unions_features_in_sorted_order() {
        let mut dependency = RuntimeDependencyContribution::new("dep").feature("server");

        dependency
            .merge(RuntimeDependencyContribution::new("dep").feature("client"))
            .unwrap();

        assert_eq!(
            dependency
                .features
                .into_iter()
                .collect::<Vec<_>>(),
            vec!["client", "server"],
        );
    }

    #[test]
    fn merge_adopts_default_features_when_unset() {
        let mut dependency = RuntimeDependencyContribution::new("dep");

        dependency
            .merge(RuntimeDependencyContribution::new("dep").default_features(false))
            .unwrap();

        assert_eq!(dependency.default_features, Some(false));
    }

    #[test]
    fn merge_allows_matching_default_features() {
        let mut dependency = RuntimeDependencyContribution::new("dep").default_features(false);

        assert!(
            dependency
                .merge(RuntimeDependencyContribution::new("dep").default_features(false))
                .is_ok()
        );
    }

    #[test]
    fn merge_rejects_conflicting_default_features() {
        let mut dependency = RuntimeDependencyContribution::new("dep").default_features(true);

        assert!(
            dependency
                .merge(RuntimeDependencyContribution::new("dep").default_features(false))
                .is_err()
        );
    }

    #[test]
    fn merge_rejects_mismatched_names() {
        let mut dependency = RuntimeDependencyContribution::new("dep");

        assert!(
            dependency
                .merge(RuntimeDependencyContribution::new("other"))
                .is_err()
        );
    }

    #[test]
    fn external_merge_unions_features() {
        let mut dependency = ExternalDependencyContribution::new("dep").feature("a");

        dependency
            .merge(ExternalDependencyContribution::new("dep").feature("b"))
            .unwrap();

        assert_eq!(
            dependency
                .features
                .into_iter()
                .collect::<Vec<_>>(),
            vec!["a", "b"],
        );
    }

    #[test]
    fn external_merge_adopts_unset_fields() {
        let mut dependency = ExternalDependencyContribution::new("dep");

        dependency
            .merge(ExternalDependencyContribution::new("dep").version("1.0"))
            .unwrap();

        assert_eq!(dependency.version.as_deref(), Some("1.0"));
    }

    #[test]
    fn external_merge_rejects_conflicting_versions() {
        let mut dependency = ExternalDependencyContribution::new("dep").version("1.0");

        assert!(
            dependency
                .merge(ExternalDependencyContribution::new("dep").version("2.0"))
                .is_err()
        );
    }
}
