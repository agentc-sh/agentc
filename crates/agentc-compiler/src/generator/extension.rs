// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::HashMap;

/// A named coordination point between blocks.
///
/// A block declares an extension point, a location in its output where
/// contributions from other blocks will be collected and resolved.
/// The point owns its reduction strategy, determining how multiple contributions
/// collapse into a single string.
#[derive(Debug, Clone)]
pub struct ExtensionPoint {
    name: String,
    contributions: Vec<String>,
    reducer: fn(Vec<String>) -> String,
}

impl ExtensionPoint {
    pub fn new(name: impl Into<String>, reducer: fn(Vec<String>) -> String) -> Self {
        Self {
            name: name.into(),
            contributions: Vec::new(),
            reducer,
        }
    }

    /// Get the name of the extension point.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the contributions made to this extension point.
    pub fn contributions(&self) -> &[String] {
        &self.contributions
    }

    /// Add a contribution to this extension point.
    pub fn contribute(&mut self, contribution: impl Into<String>) {
        self.contributions
            .push(contribution.into());
    }

    /// Reduce the contributions to a single string using the point's reduction strategy.
    pub fn reduce(self) -> String {
        (self.reducer)(self.contributions.clone())
    }
}

/// A declaration that a block intends to contribute content into a named
/// extension point.
///
/// Strict contributions require that the extension point be declared by some block,
/// while lenient silently skips if no extension block is declared.
#[derive(Debug, Clone)]
pub struct Contribution {
    pub point: String,
    pub strict: bool,
}

impl Contribution {
    /// Fails at validation time if the target extension point is not declared
    /// by any block in the graph.
    pub fn strict(point: impl Into<String>) -> Self {
        Self { point: point.into(), strict: true }
    }

    /// Silently skipped if the target extension point is not declared by any block in the graph.
    pub fn lenient(point: impl Into<String>) -> Self {
        Self { point: point.into(), strict: false }
    }
}

/// Standard reduction strategies for extension points.
pub mod reducers {
    /// Join all contributions with a newline.
    pub fn concat(contributions: Vec<String>) -> String {
        contributions.join("\n")
    }

    /// Join contributions with a newline, removing consecutive duplicates.
    pub fn concat_unique(mut contributions: Vec<String>) -> String {
        contributions.dedup();
        contributions.join("\n")
    }

    /// Join contributions with a comma.
    pub fn join_comma(contributions: Vec<String>) -> String {
        contributions.join(",")
    }

    /// Use only the last contribution, ignoring all previous ones.
    pub fn last(mut contributions: Vec<String>) -> String {
        contributions.pop().unwrap_or_default()
    }

    /// Use only the first contribution, ignoring all subsequent ones.
    pub fn first(mut contributions: Vec<String>) -> String {
        if contributions.is_empty() {
            String::new()
        } else {
            contributions.remove(0)
        }
    }
}

/// The resolved extension points for a generation run.
///
/// Once built, the registry is immutable.
#[derive(Clone)]
pub struct ExtensionRegistry {
    resolved: HashMap<String, String>,
}

impl ExtensionRegistry {
    /// Build the registry by resolving all declared extension points with their contributions.
    pub fn resolve(points: Vec<ExtensionPoint>) -> Self {
        Self {
            resolved: points
                .into_iter()
                .map(|point| (point.name.clone(), point.reduce()))
                .collect(),
        }
    }

    /// An empty registry with no resolved points.
    ///
    /// Used internally when rendering contributions, where the registry
    /// has not yet been built.
    pub fn empty() -> Self {
        Self { resolved: HashMap::new() }
    }

    /// Get the resolved content for a given extension point, if it exists.
    pub fn get(&self, point: &str) -> Option<&str> {
        self.resolved
            .get(point)
            .map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(reducer: fn(Vec<String>) -> String) -> ExtensionPoint {
        ExtensionPoint::new("test", reducer)
    }

    #[test]
    fn new_point_has_no_contributions() {
        let p = point(reducers::concat);
        assert!(p.contributions().is_empty());
    }

    #[test]
    fn contribute_appends_in_order() {
        let mut p = point(reducers::concat);
        p.contribute("first");
        p.contribute("second");
        assert_eq!(p.contributions(), &["first", "second"]);
    }

    #[test]
    fn reducer_concat_joins_with_newline() {
        let mut p = point(reducers::concat);
        p.contribute("a");
        p.contribute("b");
        assert_eq!(p.reduce(), "a\nb");
    }

    #[test]
    fn reducer_concat_empty_produces_empty_string() {
        let p = point(reducers::concat);
        assert_eq!(p.reduce(), "");
    }

    #[test]
    fn reducer_concat_unique_removes_consecutive_dupes() {
        let mut p = point(reducers::concat_unique);
        p.contribute("a");
        p.contribute("a");
        p.contribute("b");
        p.contribute("a"); // non-consecutive, kept
        assert_eq!(p.reduce(), "a\nb\na");
    }

    #[test]
    fn reducer_first_uses_only_first_contribution() {
        let mut p = point(reducers::first);
        p.contribute("first");
        p.contribute("second");
        assert_eq!(p.reduce(), "first");
    }

    #[test]
    fn reducer_first_empty_produces_empty_string() {
        let p = point(reducers::first);
        assert_eq!(p.reduce(), "");
    }

    #[test]
    fn reducer_last_uses_only_last_contribution() {
        let mut p = point(reducers::last);
        p.contribute("first");
        p.contribute("second");
        assert_eq!(p.reduce(), "second");
    }

    #[test]
    fn reducer_last_empty_produces_empty_string() {
        let p = point(reducers::last);
        assert_eq!(p.reduce(), "");
    }

    #[test]
    fn strict_sets_strict_true() {
        let c = Contribution::strict("my_point");
        assert_eq!(c.point, "my_point");
        assert!(c.strict);
    }

    #[test]
    fn lenient_sets_strict_false() {
        let c = Contribution::lenient("my_point");
        assert_eq!(c.point, "my_point");
        assert!(!c.strict);
    }

    #[test]
    fn resolve_produces_correct_values() {
        let mut p1 = ExtensionPoint::new("deps", reducers::concat);
        p1.contribute("tokio");
        p1.contribute("serde");

        let mut p2 = ExtensionPoint::new("mods", reducers::concat);
        p2.contribute("mod api;");

        let registry = ExtensionRegistry::resolve(vec![p1, p2]);
        assert_eq!(registry.get("deps"), Some("tokio\nserde"));
        assert_eq!(registry.get("mods"), Some("mod api;"));
    }

    #[test]
    fn get_unknown_point_returns_none() {
        let registry = ExtensionRegistry::resolve(vec![]);
        assert_eq!(registry.get("nonexistent"), None);
    }

    #[test]
    fn empty_registry_returns_none_for_any_point() {
        let registry = ExtensionRegistry::empty();
        assert_eq!(registry.get("anything"), None);
    }

    #[test]
    fn resolve_point_with_no_contributions_produces_empty_string() {
        let p = ExtensionPoint::new("empty_point", reducers::concat);
        let registry = ExtensionRegistry::resolve(vec![p]);
        assert_eq!(registry.get("empty_point"), Some(""));
    }
}
