// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde_json::Value;
use std::collections::BTreeMap;

use crate::{
    errors::ConfigError,
    path::{Path, Segment},
};

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigNode {
    Null,
    Scalar(String),
    Json(Value),
    Map(BTreeMap<String, ConfigNode>),
    Sequence(Vec<ConfigNode>),
}

impl ConfigNode {
    pub fn null() -> Self {
        ConfigNode::Null
    }

    pub fn scalar(value: impl Into<String>) -> Self {
        ConfigNode::Scalar(value.into())
    }

    pub fn json(value: impl Into<Value>) -> Self {
        ConfigNode::Json(value.into())
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

    pub fn kind(&self) -> &'static str {
        match self {
            ConfigNode::Null => "null",
            ConfigNode::Scalar(_) => "scalar",
            ConfigNode::Json(_) => "json",
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

    pub fn insert(&mut self, path: &[Segment], value: ConfigNode) -> Result<(), ConfigError> {
        self.insert_at(path, value, &Path::new())
    }

    pub fn insert_at(
        &mut self,
        path: &[Segment],
        value: ConfigNode,
        current_path: &Path,
    ) -> Result<(), ConfigError> {
        if path.is_empty() {
            *self = value;
            return Ok(());
        }

        match &path[0] {
            Segment::Key(key) => {
                if matches!(self, ConfigNode::Null) {
                    *self = ConfigNode::Map(BTreeMap::new());
                }

                match self {
                    ConfigNode::Map(map) => map
                        .entry(key.clone())
                        .or_insert(ConfigNode::Null)
                        .insert_at(&path[1..], value, &current_path.child(Segment::key(key))),
                    other => {
                        Err(ConfigError::merge_conflict(current_path.clone(), other.kind(), "map"))
                    }
                }
            }
            Segment::Index(idx) => {
                if matches!(self, ConfigNode::Null) {
                    *self = ConfigNode::Sequence(Vec::new());
                }

                match self {
                    ConfigNode::Sequence(sequence) => {
                        if *idx >= sequence.len() {
                            sequence.resize(*idx + 1, ConfigNode::Null);
                        }

                        sequence[*idx].insert_at(
                            &path[1..],
                            value,
                            &current_path.child(Segment::index(*idx)),
                        )
                    }
                    other => Err(ConfigError::merge_conflict(
                        current_path.clone(),
                        other.kind(),
                        "sequence",
                    )),
                }
            }
        }
    }
}
