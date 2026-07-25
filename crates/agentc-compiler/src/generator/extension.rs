// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    any::{Any, TypeId, type_name},
    collections::HashMap,
    marker::PhantomData,
};

use crate::generator::errors::GeneratorError;

/// A named coordination point between blocks.
///
/// A block declares an extension point, a location in its output where
/// contributions from other blocks will be collected and resolved.
/// The point owns its reduction strategy, determining how multiple contributions
/// collapse into a single string.
pub trait ExtensionPoint: Send + Sync {
    type Contribution: Send + Sync + 'static;

    fn name(&self) -> &str;

    fn reduce(&self, contributions: Vec<Self::Contribution>) -> Result<String, GeneratorError>;
}

pub trait ErasedExtensionPoint: Send + Sync {
    fn name(&self) -> &str;

    fn contribution_type(&self) -> TypeId;

    fn contribution_type_name(&self) -> &'static str;

    fn clone_box(&self) -> Box<dyn ErasedExtensionPoint>;

    fn reduce(&self, contributions: Vec<ErasedContributionValue>)
    -> Result<String, GeneratorError>;
}

impl<P> ErasedExtensionPoint for P
where
    P: ExtensionPoint + Clone + 'static,
{
    fn name(&self) -> &str {
        ExtensionPoint::name(self)
    }

    fn contribution_type(&self) -> TypeId {
        TypeId::of::<P::Contribution>()
    }

    fn contribution_type_name(&self) -> &'static str {
        type_name::<P::Contribution>()
    }

    fn clone_box(&self) -> Box<dyn ErasedExtensionPoint> {
        Box::new(self.clone())
    }

    fn reduce(
        &self,
        contributions: Vec<ErasedContributionValue>,
    ) -> Result<String, GeneratorError> {
        ExtensionPoint::reduce(
            self,
            contributions
                .into_iter()
                .map(ErasedContributionValue::downcast::<P::Contribution>)
                .collect::<Result<Vec<_>, _>>()?,
        )
    }
}

impl Clone for Box<dyn ErasedExtensionPoint> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub struct ErasedContributionValue(Box<dyn Any + Send + Sync>);

impl ErasedContributionValue {
    pub fn new<C>(value: C) -> Self
    where
        C: Send + Sync + 'static,
    {
        Self(Box::new(value))
    }

    pub fn downcast<C>(self) -> Result<C, GeneratorError>
    where
        C: Send + Sync + 'static,
    {
        self.0
            .downcast::<C>()
            .map(|value| *value)
            .map_err(|_| {
                GeneratorError::unexpected(format!(
                    "contribution value type mismatch; expected {}",
                    type_name::<C>(),
                ))
            })
    }
}

impl From<String> for ErasedContributionValue {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for ErasedContributionValue {
    fn from(value: &str) -> Self {
        Self::new(value.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct StringExtensionPoint {
    name: String,
    reducer: fn(Vec<String>) -> String,
}

impl StringExtensionPoint {
    pub fn new(name: impl Into<String>, reducer: fn(Vec<String>) -> String) -> Self {
        Self { name: name.into(), reducer }
    }
}

impl ExtensionPoint for StringExtensionPoint {
    type Contribution = String;

    fn name(&self) -> &str {
        &self.name
    }

    fn reduce(&self, contributions: Vec<Self::Contribution>) -> Result<String, GeneratorError> {
        Ok((self.reducer)(contributions))
    }
}

/// A declaration that a block intends to contribute content into a named
/// extension point.
///
/// Strict contributions require that the extension point be declared by some block,
/// while lenient silently skips if no extension block is declared.
#[derive(Debug, Clone)]
pub struct Contribution<C = String> {
    pub point: String,
    pub strict: bool,
    _marker: PhantomData<C>,
}

impl<C> Contribution<C>
where
    C: Send + Sync + 'static,
{
    /// Fails at validation time if the target extension point is not declared
    /// by any block in the graph.
    pub fn strict(point: impl Into<String>) -> Self {
        Self {
            point: point.into(),
            strict: true,
            _marker: PhantomData,
        }
    }

    /// Silently skipped if the target extension point is not declared by any block in the graph.
    pub fn lenient(point: impl Into<String>) -> Self {
        Self {
            point: point.into(),
            strict: false,
            _marker: PhantomData,
        }
    }

    pub fn erase(&self) -> ErasedContribution {
        ErasedContribution {
            point: self.point.clone(),
            strict: self.strict,
            contribution_type: TypeId::of::<C>(),
            contribution_type_name: type_name::<C>(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ErasedContribution {
    pub point: String,
    pub strict: bool,
    pub contribution_type: TypeId,
    pub contribution_type_name: &'static str,
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
    pub fn resolve(
        points: Vec<Box<dyn ErasedExtensionPoint>>,
        mut contributions: HashMap<String, Vec<ErasedContributionValue>>,
    ) -> Result<Self, GeneratorError> {
        Ok(Self {
            resolved: points
                .into_iter()
                .map(|point| {
                    let name = point.name().to_string();

                    point
                        .reduce(
                            contributions
                                .remove(&name)
                                .unwrap_or_default(),
                        )
                        .map(|value| (name, value))
                })
                .collect::<Result<HashMap<_, _>, _>>()?,
        })
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

    fn point(reducer: fn(Vec<String>) -> String) -> StringExtensionPoint {
        StringExtensionPoint::new("test", reducer)
    }

    /// A typed, non-string extension point used to prove that erased
    /// contributions of a custom type reduce into a final registry string.
    #[derive(Clone)]
    struct NumberPoint;

    impl ExtensionPoint for NumberPoint {
        type Contribution = u64;

        fn name(&self) -> &str {
            "numbers"
        }

        fn reduce(&self, contributions: Vec<Self::Contribution>) -> Result<String, GeneratorError> {
            Ok(contributions
                .into_iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
                .join(","))
        }
    }

    #[test]
    fn string_point_has_name() {
        let p = point(reducers::concat);
        assert_eq!(ExtensionPoint::name(&p), "test");
    }

    #[test]
    fn reducer_concat_joins_with_newline() {
        assert_eq!(
            ExtensionPoint::reduce(
                &point(reducers::concat),
                vec!["a".to_string(), "b".to_string()],
            )
            .unwrap(),
            "a\nb",
        );
    }

    #[test]
    fn reducer_concat_empty_produces_empty_string() {
        assert_eq!(ExtensionPoint::reduce(&point(reducers::concat), vec![]).unwrap(), "",);
    }

    #[test]
    fn reducer_concat_unique_removes_consecutive_dupes() {
        assert_eq!(
            ExtensionPoint::reduce(
                &point(reducers::concat_unique),
                vec![
                    "a".to_string(),
                    "a".to_string(),
                    "b".to_string(),
                    "a".to_string(),
                ],
            )
            .unwrap(),
            "a\nb\na",
        );
    }

    #[test]
    fn reducer_first_uses_only_first_contribution() {
        assert_eq!(
            ExtensionPoint::reduce(
                &point(reducers::first),
                vec!["first".to_string(), "second".to_string()],
            )
            .unwrap(),
            "first",
        );
    }

    #[test]
    fn reducer_first_empty_produces_empty_string() {
        assert_eq!(ExtensionPoint::reduce(&point(reducers::first), vec![]).unwrap(), "",);
    }

    #[test]
    fn reducer_last_uses_only_last_contribution() {
        assert_eq!(
            ExtensionPoint::reduce(
                &point(reducers::last),
                vec!["first".to_string(), "second".to_string()],
            )
            .unwrap(),
            "second",
        );
    }

    #[test]
    fn reducer_last_empty_produces_empty_string() {
        assert_eq!(ExtensionPoint::reduce(&point(reducers::last), vec![]).unwrap(), "",);
    }

    #[test]
    fn strict_sets_strict_true() {
        let c = Contribution::<String>::strict("my_point");
        assert_eq!(c.point, "my_point");
        assert!(c.strict);
    }

    #[test]
    fn lenient_sets_strict_false() {
        let c = Contribution::<String>::lenient("my_point");
        assert_eq!(c.point, "my_point");
        assert!(!c.strict);
    }

    #[test]
    fn resolve_produces_correct_values() {
        let registry = ExtensionRegistry::resolve(
            vec![
                Box::new(StringExtensionPoint::new("deps", reducers::concat)),
                Box::new(StringExtensionPoint::new("mods", reducers::concat)),
            ],
            HashMap::from([
                (
                    "deps".to_string(),
                    vec![
                        ErasedContributionValue::new("tokio".to_string()),
                        ErasedContributionValue::new("serde".to_string()),
                    ],
                ),
                ("mods".to_string(), vec![ErasedContributionValue::new("mod api;".to_string())]),
            ]),
        )
        .unwrap();

        assert_eq!(registry.get("deps"), Some("tokio\nserde"));
        assert_eq!(registry.get("mods"), Some("mod api;"));
    }

    #[test]
    fn resolve_reduces_typed_contributions_into_final_string() {
        let registry = ExtensionRegistry::resolve(
            vec![Box::new(NumberPoint)],
            HashMap::from([(
                "numbers".to_string(),
                vec![
                    ErasedContributionValue::new(1u64),
                    ErasedContributionValue::new(2u64),
                    ErasedContributionValue::new(3u64),
                ],
            )]),
        )
        .unwrap();

        assert_eq!(registry.get("numbers"), Some("1,2,3"));
    }

    #[test]
    fn get_unknown_point_returns_none() {
        let registry = ExtensionRegistry::resolve(vec![], HashMap::new()).unwrap();
        assert_eq!(registry.get("nonexistent"), None);
    }

    #[test]
    fn empty_registry_returns_none_for_any_point() {
        let registry = ExtensionRegistry::empty();
        assert_eq!(registry.get("anything"), None);
    }

    #[test]
    fn resolve_point_with_no_contributions_produces_empty_string() {
        let registry = ExtensionRegistry::resolve(
            vec![Box::new(StringExtensionPoint::new(
                "empty_point",
                reducers::concat,
            ))],
            HashMap::new(),
        )
        .unwrap();

        assert_eq!(registry.get("empty_point"), Some(""));
    }
}
