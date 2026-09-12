// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use agentc_compiler::generator::{errors::GeneratorError, extension::ExtensionPoint};

use crate::{
    contributions::set::{ContributionSet, Mergeable},
    errors::BlocksError,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ImportLeaf {
    Item {
        name: &'static str,
        alias: Option<&'static str>,
    },
    Glob,
}

impl ImportLeaf {
    fn render(&self) -> String {
        match self {
            Self::Item { name, alias: None } => name.to_string(),
            Self::Item {
                name,
                alias: Some(alias),
            } => format!("{name} as {alias}"),
            Self::Glob => "*".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportContribution {
    path: &'static [&'static str],
    leaves: BTreeSet<ImportLeaf>,
}

impl ImportContribution {
    pub fn path(path: &'static [&'static str]) -> Self {
        Self {
            path,
            leaves: BTreeSet::new(),
        }
    }

    fn render(&self) -> String {
        format!(
            "use {}{};",
            self.path
                .iter()
                .map(|segment| format!("{segment}::"))
                .collect::<String>(),
            match self.leaves.len() {
                1 => self
                    .leaves
                    .iter()
                    .map(ImportLeaf::render)
                    .collect::<String>(),
                _ => format!(
                    "{{{}}}",
                    self.leaves
                        .iter()
                        .map(ImportLeaf::render)
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
            },
        )
    }

    pub fn item(mut self, name: &'static str) -> Self {
        self.leaves.insert(ImportLeaf::Item { name, alias: None });
        self
    }

    pub fn item_as(mut self, name: &'static str, alias: &'static str) -> Self {
        self.leaves.insert(ImportLeaf::Item {
            name,
            alias: Some(alias),
        });
        self
    }

    pub fn glob(mut self) -> Self {
        self.leaves.insert(ImportLeaf::Glob);
        self
    }
}

impl Mergeable for ImportContribution {
    type Key = &'static [&'static str];

    fn key(&self) -> Self::Key {
        self.path
    }

    fn merge(&mut self, other: Self) -> Result<(), BlocksError> {
        self.leaves.extend(other.leaves);

        Ok(())
    }
}

pub type Imports = ContributionSet<ImportContribution>;

#[derive(Debug, Clone)]
pub struct ImportsExtensionPoint {
    name: &'static str,
}

impl ImportsExtensionPoint {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl ExtensionPoint for ImportsExtensionPoint {
    type Contribution = Imports;

    fn name(&self) -> &str {
        self.name
    }

    fn reduce(&self, contributions: Vec<Self::Contribution>) -> Result<String, GeneratorError> {
        Ok(
            Imports::merge_all(contributions)
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?
                .into_values()
                .filter(|import| !import.leaves.is_empty())
                .map(|import| import.render())
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ImportsFixture;

    impl ImportsFixture {
        fn render(contributions: impl IntoIterator<Item = Vec<ImportContribution>>) -> String {
            ExtensionPoint::reduce(
                &ImportsExtensionPoint::new("agent::use"),
                contributions
                    .into_iter()
                    .map(|entries| {
                        Imports::from_entries(entries).expect("entries should merge")
                    })
                    .collect(),
            )
            .expect("imports should render")
        }
    }

    #[test]
    fn the_same_import_from_two_contributions_renders_once() {
        assert_eq!(
            ImportsFixture::render([
                vec![ImportContribution::path(&["a"]).item_as("F", "_")],
                vec![ImportContribution::path(&["a"]).item_as("F", "_")],
            ]),
            "use a::F as _;",
        );
    }

    #[test]
    fn imports_from_the_same_path_combine() {
        assert_eq!(
            ImportsFixture::render([
                vec![
                    ImportContribution::path(&["a"])
                        .item("F")
                        .item("Q")
                        .item("R"),
                ],
                vec![ImportContribution::path(&["a"]).item("T")],
            ]),
            "use a::{F, Q, R, T};",
        );
    }

    #[test]
    fn the_same_name_from_two_paths_is_two_imports() {
        assert_eq!(
            ImportsFixture::render([
                vec![ImportContribution::path(&["a"]).item("F")],
                vec![ImportContribution::path(&["b"]).item("F")],
            ]),
            "use a::F;\nuse b::F;",
        );
    }

    #[test]
    fn the_same_name_under_two_aliases_is_two_imports() {
        assert_eq!(
            ImportsFixture::render([
                vec![ImportContribution::path(&["a"]).item("F")],
                vec![ImportContribution::path(&["a"]).item_as("F", "F2")],
            ]),
            "use a::{F, F as F2};",
        );
    }

    #[test]
    fn a_glob_combines_with_items() {
        assert_eq!(
            ImportsFixture::render([
                vec![ImportContribution::path(&["a"]).glob()],
                vec![ImportContribution::path(&["a"]).item("F")],
            ]),
            "use a::{F, *};",
        );
    }

    #[test]
    fn every_segment_of_the_path_renders() {
        assert_eq!(
            ImportsFixture::render([
                vec![
                    ImportContribution::path(&["a", "b", "c"])
                        .item("F")
                        .item("Q"),
                ],
                vec![ImportContribution::path(&["a", "b", "c"]).item("R")],
            ]),
            "use a::b::c::{F, Q, R};",
        );
    }

    #[test]
    fn a_parent_path_is_a_different_import() {
        assert_eq!(
            ImportsFixture::render([
                vec![ImportContribution::path(&["a", "b"]).item("F")],
                vec![ImportContribution::path(&["a"]).item("F")],
            ]),
            "use a::F;\nuse a::b::F;",
        );
    }

    #[test]
    fn an_empty_path_renders_the_leaf_alone() {
        assert_eq!(
            ImportsFixture::render([vec![ImportContribution::path(&[]).item("a")]]),
            "use a;",
        );
    }

    #[test]
    fn a_contribution_without_leaves_renders_nothing() {
        assert_eq!(
            ImportsFixture::render([vec![ImportContribution::path(&["a"])]]),
            "",
        );
    }

    #[test]
    fn imports_render_in_path_order() {
        assert_eq!(
            ImportsFixture::render([vec![
                ImportContribution::path(&["crate", "config"]).item("F"),
                ImportContribution::path(&["b"]).item("F"),
                ImportContribution::path(&["a", "c"]).item("F"),
            ]]),
            "use a::c::F;\nuse b::F;\nuse crate::config::F;",
        );
    }

    #[test]
    fn no_contributions_render_nothing() {
        assert_eq!(ImportsFixture::render([]), "");
    }
}
