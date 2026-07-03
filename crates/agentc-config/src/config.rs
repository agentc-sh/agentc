// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use futures::future::join_all;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{
    de::from_node,
    errors::ConfigError,
    node::ConfigNode,
    path::Path,
    traits::{Mapper, Source},
};

pub struct Config {
    root: ConfigNode,
}

impl Config {
    pub fn new(root: ConfigNode) -> Self {
        Self { root }
    }

    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    pub fn root(&self) -> &ConfigNode {
        &self.root
    }

    pub fn get<T: DeserializeOwned>(&self, path: impl Into<Path>) -> Result<T, ConfigError> {
        from_node(
            self.root
                .get(&path.into())
                .unwrap_or(&ConfigNode::Null),
        )
    }

    pub fn get_node(&self, path: impl Into<Path>) -> Option<&ConfigNode> {
        self.root.get(&path.into())
    }

    pub fn try_deserialize<T: DeserializeOwned>(&self) -> Result<T, ConfigError> {
        from_node(&self.root)
    }
}

pub struct ConfigBuilder {
    constants: Vec<(Path, Value)>,
    defaults: Vec<(Path, Value)>,
    sources: Vec<Box<dyn Source>>,
    mappers: Vec<Box<dyn Mapper>>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            defaults: Vec::new(),
            sources: Vec::new(),
            mappers: Vec::new(),
        }
    }

    pub fn constant(mut self, path: impl Into<Path>, value: impl Into<Value>) -> Self {
        self.constants
            .push((path.into(), value.into()));
        self
    }

    pub fn default(mut self, path: impl Into<Path>, value: impl Into<Value>) -> Self {
        self.defaults
            .push((path.into(), value.into()));
        self
    }

    pub fn source(mut self, source: impl Source + 'static) -> Self {
        self.sources.push(Box::new(source));
        self
    }

    pub fn mapper(mut self, mapper: impl Mapper + 'static) -> Self {
        self.mappers.push(Box::new(mapper));
        self
    }

    pub async fn build(self) -> Result<Config, ConfigError> {
        let mut root = ConfigNode::null();

        for (path, value) in self.defaults {
            root.insert(&path, ConfigNode::json(value))?;
        }

        let collected = join_all(
            self.sources
                .iter()
                .map(|source| async move { (source.name().to_string(), source.vars().await) }),
        )
        .await;

        let mut vars: Vec<(String, String)> = Vec::new();

        for (name, result) in collected {
            vars.extend(result.map_err(|e| ConfigError::source(name, e))?);
        }

        for mapper in &self.mappers {
            for (path, node) in mapper
                .map(&vars)
                .map_err(|e| ConfigError::source(mapper.name(), e))?
            {
                root.insert(&path, node)?;
            }
        }

        for (path, value) in self.constants {
            root.insert(&path, ConfigNode::json(value))?;
        }

        Ok(Config::new(root))
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
