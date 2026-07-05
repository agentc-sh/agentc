// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    any::TypeId,
    collections::HashMap,
};

pub trait GenerationFeature: Send + Sync + 'static {
    const NAME: &'static str;
}

#[derive(Debug, Clone, Default)]
pub struct GenerationFeatureSet {
    names: HashMap<TypeId, &'static str>,
}

impl GenerationFeatureSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert<T>(&mut self)
    where
        T: GenerationFeature,
    {
        self.names
            .insert(TypeId::of::<T>(), T::NAME);
    }

    pub fn contains<T>(&self) -> bool
    where
        T: GenerationFeature,
    {
        self.names.contains_key(&TypeId::of::<T>())
    }

    pub fn union(&self, other: &Self) -> Self {
        let mut union = self.clone();

        union.names.extend(other.names.iter().map(|(id, name)| (*id, *name)));
        union
    }

    pub fn missing_requirements(&self, provided: &Self) -> Vec<&'static str> {
        let mut missing = self
            .names
            .iter()
            .filter_map(|(id, name)| (!provided.names.contains_key(id)).then_some(*name))
            .collect::<Vec<_>>();

        missing.sort_unstable();
        missing
    }

    pub fn names(&self) -> Vec<&'static str> {
        let mut names = self
            .names
            .values()
            .copied()
            .collect::<Vec<_>>();

        names.sort_unstable();
        names
    }
}

macro_rules! builtin_feature {
    ($name:ident, $label:literal) => {
        pub struct $name;

        impl GenerationFeature for $name {
            const NAME: &'static str = $label;
        }
    };
}

builtin_feature!(Cli, "cli");
builtin_feature!(LongLivedProcess, "long_lived_process");
builtin_feature!(HttpServer, "http_server");
builtin_feature!(Streaming, "streaming");
builtin_feature!(AgUiService, "ag_ui_service");

#[cfg(test)]
mod tests {
    use super::*;

    struct TestFeature;

    impl GenerationFeature for TestFeature {
        const NAME: &'static str = "test_feature";
    }

    #[test]
    fn membership_is_type_based() {
        let mut features = GenerationFeatureSet::new();

        features.insert::<Cli>();
        features.insert::<TestFeature>();

        assert!(features.contains::<Cli>());
        assert!(features.contains::<TestFeature>());
        assert!(!features.contains::<Streaming>());
    }

    #[test]
    fn union_merges_feature_sets() {
        let mut left = GenerationFeatureSet::new();
        let mut right = GenerationFeatureSet::new();

        left.insert::<Cli>();
        right.insert::<Streaming>();

        let union = left.union(&right);

        assert!(union.contains::<Cli>());
        assert!(union.contains::<Streaming>());
    }

    #[test]
    fn missing_requirements_are_sorted_for_diagnostics() {
        let mut required = GenerationFeatureSet::new();
        let mut provided = GenerationFeatureSet::new();

        required.insert::<Streaming>();
        required.insert::<Cli>();
        provided.insert::<Cli>();

        assert_eq!(required.missing_requirements(&provided), vec!["streaming"]);
    }
}
