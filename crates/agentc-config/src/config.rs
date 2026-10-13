// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use futures::future::join_all;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{
    de::from_node,
    declarations::Declarations,
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
    declarations: Declarations,
    sources: Vec<Box<dyn Source>>,
    mappers: Vec<Box<dyn Mapper>>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self::from(Declarations::new())
    }

    pub fn default(self, path: impl Into<Path>, value: impl Into<Value>) -> Self {
        Self {
            declarations: self.declarations.default(path, value),
            ..self
        }
    }

    pub fn constant(self, path: impl Into<Path>, value: impl Into<Value>) -> Self {
        Self {
            declarations: self.declarations.constant(path, value),
            ..self
        }
    }

    pub fn seal(self, path: impl Into<Path>) -> Self {
        Self {
            declarations: self.declarations.seal(path),
            ..self
        }
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

        let opens = self
            .mappers
            .iter()
            .flat_map(|mapper| mapper.targets())
            .collect::<Vec<_>>();

        let mut mapped = Vec::new();

        for mapper in &self.mappers {
            mapped.extend(
                mapper
                    .map(&vars)
                    .map_err(|e| ConfigError::source(mapper.name(), e))?,
            );
        }

        mapped.sort_by(|left, right| {
            left.path
                .len()
                .cmp(&right.path.len())
                .then_with(|| left.path.cmp(&right.path))
        });

        Ok(
            Config::new(
                self.declarations
                    .build(&mapped, &opens)?,
            )
        )
    }
}

impl From<Declarations> for ConfigBuilder {
    fn from(declarations: Declarations) -> Self {
        Self {
            declarations,
            sources: Vec::new(),
            mappers: Vec::new(),
        }
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
