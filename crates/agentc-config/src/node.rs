// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde_json::{Number, Value};
use std::{borrow::Cow, collections::BTreeMap};

use crate::{
    errors::ConfigError,
    path::{Path, Segment},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    Raw(String),
    String(String),
    Number(Number),
    Bool(bool),
}

impl Scalar {
    pub fn as_text(&self) -> Cow<'_, str> {
        match self {
            Self::Raw(text) | Self::String(text) => Cow::Borrowed(text),
            Self::Number(number) => Cow::Owned(number.to_string()),
            Self::Bool(value) => Cow::Owned(value.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigNode {
    Null,
    Scalar(Scalar),
    Map(BTreeMap<String, ConfigNode>),
    Sequence(Vec<ConfigNode>),
}

impl ConfigNode {
    pub fn null() -> Self {
        ConfigNode::Null
    }

    pub fn scalar(value: impl Into<String>) -> Self {
        ConfigNode::Scalar(Scalar::Raw(value.into()))
    }

    pub fn map(entries: impl IntoIterator<Item = (impl Into<String>, ConfigNode)>) -> Self {
        ConfigNode::Map(
            entries
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect(),
        )
    }

    pub fn sequence(items: impl IntoIterator<Item = ConfigNode>) -> Self {
        ConfigNode::Sequence(items.into_iter().collect())
    }

    pub fn document(raw: &str, path: &Path) -> Result<Self, ConfigError> {
        serde_json::from_str::<Value>(raw)
            .map(ConfigNode::from)
            .map_err(|error| ConfigError::invalid_document(path.clone(), error.to_string()))
    }

    fn expand(&mut self, segment: &Segment, current_path: &Path) -> Result<(), ConfigError> {
        if let ConfigNode::Scalar(Scalar::Raw(raw)) = self {
            *self = ConfigNode::document(raw, current_path)?;
        }

        if matches!(self, ConfigNode::Null) {
            *self = match segment {
                Segment::Key(_) => ConfigNode::Map(BTreeMap::new()),
                Segment::Index(_) => ConfigNode::Sequence(Vec::new()),
            };
        }

        Ok(())
    }

    fn child_mut(
        &mut self,
        segment: &Segment,
        current_path: &Path,
        indexes: &mut SequenceIndexes,
    ) -> Result<(Segment, &mut ConfigNode), ConfigError> {
        match (segment, self) {
            (Segment::Key(key), ConfigNode::Map(map)) => Ok(
                (
                    segment.clone(),
                    map.entry(key.clone())
                        .or_insert(ConfigNode::Null),
                )
            ),
            (Segment::Index(requested), ConfigNode::Sequence(sequence)) => {
                let index = indexes.resolve(current_path, *requested, sequence.len());

                if index == sequence.len() {
                    sequence.push(ConfigNode::Null);
                }

                Ok((Segment::index(index), &mut sequence[index]))
            }
            (Segment::Key(_), other) => {
                Err(ConfigError::merge_conflict(current_path.clone(), other.kind(), "map"))
            }
            (Segment::Index(_), other) => {
                Err(ConfigError::merge_conflict(current_path.clone(), other.kind(), "sequence"))
            }
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            ConfigNode::Null => "null",
            ConfigNode::Scalar(_) => "scalar",
            ConfigNode::Map(_) => "map",
            ConfigNode::Sequence(_) => "sequence",
        }
    }

    pub fn get(&self, path: &[Segment]) -> Option<&ConfigNode> {
        if path.is_empty() {
            return Some(self);
        }

        match (&path[0], self) {
            (Segment::Key(key), ConfigNode::Map(map)) => map.get(key)?.get(&path[1..]),
            (Segment::Index(idx), ConfigNode::Sequence(seq)) => seq.get(*idx)?.get(&path[1..]),
            _ => None,
        }
    }

    pub fn insert(
        &mut self,
        path: &[Segment],
        value: ConfigNode,
        indexes: &mut SequenceIndexes,
    ) -> Result<(), ConfigError> {
        self.insert_at(path, value, &Path::new(), indexes)
    }

    pub fn insert_at(
        &mut self,
        path: &[Segment],
        value: ConfigNode,
        current_path: &Path,
        indexes: &mut SequenceIndexes,
    ) -> Result<(), ConfigError> {
        let [head, rest @ ..] = path else {
            *self = value;

            return Ok(());
        };

        self.expand(head, current_path)?;

        let (segment, child) = self.child_mut(head, current_path, indexes)?;

        child.insert_at(rest, value, &current_path.child(segment), indexes)
    }

    pub fn remove(
        &mut self,
        path: &[Segment],
        indexes: &mut SequenceIndexes,
    ) -> Result<(), ConfigError> {
        self.remove_at(path, &Path::new(), indexes)
    }

    pub fn remove_at(
        &mut self,
        path: &[Segment],
        current_path: &Path,
        indexes: &mut SequenceIndexes,
    ) -> Result<(), ConfigError> {
        match path {
            [] => {
                *self = ConfigNode::Null;

                Ok(())
            }
            [Segment::Key(key)] => {
                self.expand(&path[0], current_path)?;

                if let ConfigNode::Map(map) = self {
                    map.remove(key);
                }

                Ok(())
            }
            [Segment::Index(_)] => {
                Err(ConfigError::positional_unset(current_path.child(path[0].clone())))
            }
            [head, rest @ ..] => {
                self.expand(head, current_path)?;

                let (segment, child) = self.child_mut(head, current_path, indexes)?;

                child.remove_at(rest, &current_path.child(segment), indexes)
            }
        }
    }

    pub fn decode_at(&mut self, path: &[Segment], current_path: &Path) -> Result<(), ConfigError> {
        if let ConfigNode::Scalar(Scalar::Raw(raw)) = self {
            if !path.is_empty() || raw.trim_start().starts_with('{') {
                *self = ConfigNode::document(raw, current_path)?;
            }
        }

        let [head, rest @ ..] = path else {
            return Ok(());
        };

        match (head, self) {
            (Segment::Key(key), ConfigNode::Map(map)) => match map.get_mut(key) {
                Some(child) => child.decode_at(rest, &current_path.child(head.clone())),
                None => Ok(()),
            },
            (Segment::Index(index), ConfigNode::Sequence(items)) => match items.get_mut(*index) {
                Some(child) => child.decode_at(rest, &current_path.child(head.clone())),
                None => Ok(()),
            },
            _ => Ok(()),
        }
    }

    pub fn as_text(&self) -> Option<Cow<'_, str>> {
        match self {
            ConfigNode::Scalar(scalar) => Some(scalar.as_text()),
            _ => None,
        }
    }
}

impl From<Value> for ConfigNode {
    fn from(value: Value) -> Self {
        match value {
            Value::Null => ConfigNode::Null,
            Value::Bool(value) => ConfigNode::Scalar(Scalar::Bool(value)),
            Value::Number(value) => ConfigNode::Scalar(Scalar::Number(value)),
            Value::String(value) => ConfigNode::Scalar(Scalar::String(value)),
            Value::Array(items) => ConfigNode::sequence(
                items
                    .into_iter()
                    .map(ConfigNode::from),
            ),
            Value::Object(entries) => ConfigNode::map(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, ConfigNode::from(value))),
            ),
        }
    }
}

#[derive(Debug, Default)]
pub struct SequenceIndexes {
    lengths: BTreeMap<Path, usize>,
    appended: BTreeMap<(Path, usize), usize>,
}

impl SequenceIndexes {
    pub fn resolve(&mut self, sequence: &Path, requested: usize, len: usize) -> usize {
        match requested
            < *self
                .lengths
                .entry(sequence.clone())
                .or_insert(len)
        {
            true => requested,
            false => *self
                .appended
                .entry((sequence.clone(), requested))
                .or_insert(len),
        }
    }
}
