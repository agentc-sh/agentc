// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use crate::errors::BlocksError;

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
