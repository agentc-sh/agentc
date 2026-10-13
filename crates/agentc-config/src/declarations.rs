// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use serde_json::Value;

use crate::{
    errors::ConfigError,
    node::{ConfigNode, SequenceIndexes},
    path::{Path, Segment},
    traits::{MappedValue, MappedVar},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum VariantTag {
    Key,
    Field(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VariantCondition {
    path: Path,
    tag: VariantTag,
    name: String,
}

impl VariantCondition {
    pub fn new(path: impl Into<Path>, tag: VariantTag, name: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            tag,
            name: name.into(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn holds(&self, root: &mut ConfigNode) -> Result<bool, ConfigError> {
        root.decode_at(&self.path, &Path::new())?;

        Ok(match &self.tag {
            VariantTag::Key => root
                .get(&self.path)
                .is_some_and(|node| {
                    node.get(&[Segment::key(self.name.clone())])
                        .is_some()
                        || node
                            .as_text()
                            .is_some_and(|text| text == self.name)
                }),
            VariantTag::Field(tag) => root
                .get(
                    &self
                        .path
                        .child(Segment::key(tag.clone())),
                )
                .and_then(ConfigNode::as_text)
                .is_some_and(|text| text == self.name),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Declarations {
    defaults: Vec<(Path, Value)>,
    seals: Vec<Path>,
    variants: Vec<(VariantCondition, Declarations)>,
}

impl Declarations {
    pub fn new() -> Self {
        Default::default()
    }

    fn is_sealed(&self, path: &Path, opens: &[Path]) -> bool {
        self.seals
            .iter()
            .filter(|seal| path.starts_with(seal))
            .map(|seal| seal.len())
            .max()
            > opens
                .iter()
                .filter(|open| path.starts_with(open))
                .map(|open| open.len())
                .max()
    }

    fn active(&self, selection: &BTreeSet<VariantCondition>) -> Declarations {
        let mut active = Declarations {
            defaults: self.defaults.clone(),
            seals: self.seals.clone(),
            variants: Vec::new(),
        };

        for (condition, declarations) in &self.variants {
            if selection.contains(condition) {
                let selected = declarations.active(selection);

                active
                    .defaults
                    .extend(selected.defaults);
                active.seals.extend(selected.seals);
            }
        }

        active
    }

    fn select(
        &self,
        root: &mut ConfigNode,
        selection: &mut BTreeSet<VariantCondition>,
    ) -> Result<(), ConfigError> {
        for (condition, declarations) in &self.variants {
            if condition.holds(root)? {
                selection.insert(condition.clone());
                declarations.select(root, selection)?;
            }
        }

        Ok(())
    }

    fn apply(&self, mapped: &[MappedVar], opens: &[Path]) -> Result<ConfigNode, ConfigError> {
        let mut root = ConfigNode::null();
        let mut defaults = self.defaults.clone();

        defaults.sort_by_key(|(path, _)| path.len());

        let mut default_indexes = SequenceIndexes::default();

        for (path, value) in defaults {
            root.insert(&path, ConfigNode::from(value), &mut default_indexes)?;
        }

        let mut mapped_indexes = SequenceIndexes::default();

        for var in mapped {
            if self.is_sealed(&var.path, opens) {
                return Err(ConfigError::sealed(var.path.clone()));
            }

            match &var.value {
                MappedValue::Set(node) => {
                    root.insert(&var.path, node.clone(), &mut mapped_indexes)?
                }
                MappedValue::Unset => root.remove(&var.path, &mut mapped_indexes)?,
            }
        }

        Ok(root)
    }

    pub fn default(mut self, path: impl Into<Path>, value: impl Into<Value>) -> Self {
        self.defaults
            .push((path.into(), value.into()));
        self
    }

    pub fn constant(self, path: impl Into<Path>, value: impl Into<Value>) -> Self {
        let path = path.into();

        self.default(path.clone(), value)
            .seal(path)
    }

    pub fn seal(mut self, path: impl Into<Path>) -> Self {
        self.seals.push(path.into());
        self
    }

    pub fn variant(
        mut self,
        condition: VariantCondition,
        declare: impl FnOnce(Declarations) -> Declarations,
    ) -> Self {
        self.variants
            .push((condition, declare(Declarations::new())));
        self
    }

    pub fn build(&self, mapped: &[MappedVar], opens: &[Path]) -> Result<ConfigNode, ConfigError> {
        let mut selection = BTreeSet::new();

        loop {
            let mut root = self
                .active(&selection)
                .apply(mapped, opens)?;
            let mut selected = BTreeSet::new();

            self.select(&mut root, &mut selected)?;

            if selected == selection {
                return Ok(root);
            }

            if let Some(dropped) = selection.difference(&selected).next() {
                return Err(ConfigError::unstable_variant(dropped.path().clone()));
            }

            selection = selected;
        }
    }
}
