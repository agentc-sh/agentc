// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use crate::errors::BlocksError;

#[derive(Debug, Clone)]
pub enum CargoDependencyContribution {
    Raw(String),
    Runtime(RuntimeDependencyContribution),
    External(ExternalDependencyContribution),
}

impl CargoDependencyContribution {
    pub fn raw(value: impl Into<String>) -> Self {
        Self::Raw(value.into())
    }

    pub fn runtime(dependency: impl Into<RuntimeDependencyContribution>) -> Self {
        Self::Runtime(dependency.into())
    }

    pub fn external(dependency: impl Into<ExternalDependencyContribution>) -> Self {
        Self::External(dependency.into())
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
    pub name: &'static str,
    pub version: Option<&'static str>,
    pub git: Option<&'static str>,
    pub branch: Option<&'static str>,
    pub tag: Option<&'static str>,
    pub rev: Option<&'static str>,
    pub path: Option<&'static str>,
    pub default_features: Option<bool>,
    pub features: BTreeSet<&'static str>,
}

impl ExternalDependencyContribution {
    fn merge_field(
        name: &'static str,
        field: &'static str,
        left: &mut Option<&'static str>,
        right: Option<&'static str>,
    ) -> Result<(), BlocksError> {
        match (*left, right) {
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

    pub fn new(name: &'static str) -> Self {
        Self {
            name,
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

    pub fn version(mut self, version: &'static str) -> Self {
        self.version = Some(version);
        self
    }

    pub fn git(mut self, url: &'static str) -> Self {
        self.git = Some(url);
        self
    }

    pub fn branch(mut self, branch: &'static str) -> Self {
        self.branch = Some(branch);
        self
    }

    pub fn tag(mut self, tag: &'static str) -> Self {
        self.tag = Some(tag);
        self
    }

    pub fn rev(mut self, rev: &'static str) -> Self {
        self.rev = Some(rev);
        self
    }

    pub fn path(mut self, path: &'static str) -> Self {
        self.path = Some(path);
        self
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
                "cannot merge external dependencies '{}' and '{}'",
                self.name, other.name,
            )));
        }

        Self::merge_field(self.name, "version", &mut self.version, other.version)?;
        Self::merge_field(self.name, "git", &mut self.git, other.git)?;
        Self::merge_field(self.name, "branch", &mut self.branch, other.branch)?;
        Self::merge_field(self.name, "tag", &mut self.tag, other.tag)?;
        Self::merge_field(self.name, "rev", &mut self.rev, other.rev)?;
        Self::merge_field(self.name, "path", &mut self.path, other.path)?;

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
}
