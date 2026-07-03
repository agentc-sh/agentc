// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::collections::HashSet;

use crate::{
    errors::SourceError,
    node::ConfigNode,
    path::{Path, Segment},
};

#[async_trait]
pub trait Source: Send + Sync {
    fn name(&self) -> &str;
    async fn vars(&self) -> Result<Vec<(String, String)>, SourceError>;
}

pub trait Mapper: Send + Sync {
    fn name(&self) -> &str;
    fn map(&self, vars: &[(String, String)]) -> Result<Vec<(Path, ConfigNode)>, SourceError>;
}

pub struct OsEnvSource;

#[async_trait]
impl Source for OsEnvSource {
    fn name(&self) -> &str {
        "os_env"
    }

    async fn vars(&self) -> Result<Vec<(String, String)>, SourceError> {
        Ok(std::env::vars().collect())
    }
}

pub struct FieldMapping {
    pub target: Path,
    pub prefix: String,
}

pub struct PrefixMapper {
    prefix: String,
    delimiter: String,
    fields: Vec<FieldMapping>,
}

impl PrefixMapper {
    pub fn new(prefix: impl Into<String>, delimiter: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            delimiter: delimiter.into(),
            fields: Vec::new(),
        }
    }

    pub fn field(mut self, target: impl Into<Path>, prefix: impl Into<String>) -> Self {
        self.fields.push(FieldMapping {
            target: target.into(),
            prefix: prefix.into(),
        });
        self
    }

    fn parse_into(
        &self,
        key: &str,
        value: &str,
        env_prefix: &str,
        target: &Path,
    ) -> Option<(Path, ConfigNode)> {
        let stripped = if key == env_prefix {
            ""
        } else {
            key.strip_prefix(env_prefix)
                .and_then(|s| s.strip_prefix(&self.delimiter))?
        };

        let segments = if stripped.is_empty() {
            Vec::new()
        } else {
            stripped
                .split(&self.delimiter)
                .map(|part| {
                    if let Ok(idx) = part.parse::<usize>() {
                        Segment::index(idx)
                    } else {
                        Segment::key(part.to_ascii_lowercase())
                    }
                })
                .collect::<Vec<_>>()
        };

        Some((
            target
                .iter()
                .cloned()
                .chain(segments)
                .collect::<Vec<_>>()
                .into(),
            ConfigNode::scalar(value),
        ))
    }
}

impl Mapper for PrefixMapper {
    fn name(&self) -> &str {
        "prefix_mapper"
    }

    fn map(&self, vars: &[(String, String)]) -> Result<Vec<(Path, ConfigNode)>, SourceError> {
        let mut mapped = Vec::new();

        let claimed = self
            .fields
            .iter()
            .map(|field| field.prefix.as_str())
            .collect::<HashSet<_>>();

        for (key, value) in vars {
            if claimed.iter().any(|prefix| {
                key.as_str() == *prefix || key.starts_with(&format!("{}{}", prefix, self.delimiter))
            }) {
                continue;
            }

            if let Some(pair) = self.parse_into(key, value, &self.prefix, &Path::new()) {
                mapped.push(pair);
            }
        }

        for field in &self.fields {
            for (key, value) in vars {
                if let Some(pair) = self.parse_into(key, value, &field.prefix, &field.target) {
                    mapped.push(pair);
                }
            }
        }

        Ok(mapped)
    }
}
