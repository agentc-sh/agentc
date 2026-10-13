// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::collections::HashSet;

use crate::{
    config::ConfigBuilder,
    declarations::Declarations,
    errors::SourceError,
    node::ConfigNode,
    path::{Path, Segment},
};

#[async_trait]
pub trait Source: Send + Sync {
    fn name(&self) -> &str;
    async fn vars(&self) -> Result<Vec<(String, String)>, SourceError>;
}

#[derive(Clone)]
pub enum MappedValue {
    Set(ConfigNode),
    Unset,
}

#[derive(Clone)]
pub struct MappedVar {
    pub path: Path,
    pub value: MappedValue,
}

pub trait Mapper: Send + Sync {
    fn name(&self) -> &str;
    fn map(&self, vars: &[(String, String)]) -> Result<Vec<MappedVar>, SourceError>;
    fn targets(&self) -> Vec<Path>;
}

pub trait ConfigSchema {
    fn declare(declarations: Declarations, path: &Path) -> Declarations;

    fn schema() -> ConfigBuilder {
        ConfigBuilder::from(Self::declare(Declarations::new(), &Path::new()))
    }
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

pub struct AliasMapping {
    pub target: Path,
    pub prefix: String,
}

pub struct PrefixMapper {
    prefix: String,
    delimiter: String,
    aliases: Vec<AliasMapping>,
}

impl PrefixMapper {
    pub fn new(prefix: impl Into<String>, delimiter: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            delimiter: delimiter.into(),
            aliases: Vec::new(),
        }
    }

    pub fn alias(mut self, target: impl Into<Path>, prefix: impl Into<String>) -> Self {
        self.aliases.push(AliasMapping {
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
    ) -> Option<MappedVar> {
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

        Some(MappedVar {
            path: target
                .iter()
                .cloned()
                .chain(segments)
                .collect::<Vec<_>>()
                .into(),
            value: if value.is_empty() {
                MappedValue::Unset
            } else {
                MappedValue::Set(ConfigNode::scalar(value))
            },
        })
    }
}

impl Mapper for PrefixMapper {
    fn name(&self) -> &str {
        "prefix_mapper"
    }

    fn map(&self, vars: &[(String, String)]) -> Result<Vec<MappedVar>, SourceError> {
        let mut mapped = Vec::new();

        let claimed = self
            .aliases
            .iter()
            .map(|alias| alias.prefix.as_str())
            .collect::<HashSet<_>>();

        for (key, value) in vars {
            if claimed.iter().any(|prefix| {
                key.as_str() == *prefix || key.starts_with(&format!("{}{}", prefix, self.delimiter))
            }) {
                continue;
            }

            if let Some(var) = self.parse_into(key, value, &self.prefix, &Path::new()) {
                mapped.push(var);
            }
        }

        for alias in &self.aliases {
            for (key, value) in vars {
                if let Some(var) = self.parse_into(key, value, &alias.prefix, &alias.target) {
                    mapped.push(var);
                }
            }
        }

        Ok(mapped)
    }

    fn targets(&self) -> Vec<Path> {
        self.aliases
            .iter()
            .map(|alias| alias.target.clone())
            .collect()
    }
}
