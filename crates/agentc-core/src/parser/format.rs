// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT
#![allow(unused)]

use hcl::{Body, from_body};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt::{Debug, Formatter, Result as FmtResult},
    path::Path,
    sync::Arc,
};

use config::{FileStoredFormat, Format, Map, Value};

use crate::parser::{errors::ParserError, middleware::traits::FormatMiddleware};

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpecFormatKind {
    #[default]
    Hcl,
    Yaml,
    Json,
}

#[derive(Clone)]
pub struct SpecFormat {
    kind: SpecFormatKind,

    hcl_deserialize_middleware: Vec<Arc<dyn FormatMiddleware<hcl::Body>>>,
    hcl_serialize_middleware: Vec<Arc<dyn FormatMiddleware<hcl::Body>>>,
    json_deserialize_middleware: Vec<Arc<dyn FormatMiddleware<serde_json::Value>>>,
    json_serialize_middleware: Vec<Arc<dyn FormatMiddleware<serde_json::Value>>>,
    yaml_deserialize_middleware: Vec<Arc<dyn FormatMiddleware<serde_norway::Value>>>,
    yaml_serialize_middleware: Vec<Arc<dyn FormatMiddleware<serde_norway::Value>>>,
}

impl SpecFormat {
    pub fn hcl() -> Self {
        Self {
            kind: SpecFormatKind::Hcl,
            hcl_deserialize_middleware: Vec::new(),
            hcl_serialize_middleware: Vec::new(),
            json_deserialize_middleware: Vec::new(),
            json_serialize_middleware: Vec::new(),
            yaml_deserialize_middleware: Vec::new(),
            yaml_serialize_middleware: Vec::new(),
        }
    }

    pub fn json() -> Self {
        Self {
            kind: SpecFormatKind::Json,
            hcl_deserialize_middleware: Vec::new(),
            hcl_serialize_middleware: Vec::new(),
            json_deserialize_middleware: Vec::new(),
            json_serialize_middleware: Vec::new(),
            yaml_deserialize_middleware: Vec::new(),
            yaml_serialize_middleware: Vec::new(),
        }
    }

    pub fn yaml() -> Self {
        Self {
            kind: SpecFormatKind::Yaml,
            hcl_deserialize_middleware: Vec::new(),
            hcl_serialize_middleware: Vec::new(),
            json_deserialize_middleware: Vec::new(),
            json_serialize_middleware: Vec::new(),
            yaml_deserialize_middleware: Vec::new(),
            yaml_serialize_middleware: Vec::new(),
        }
    }

    pub fn with_hcl_deserialize_middleware<T>(mut self, middleware: T) -> Self
    where
        T: FormatMiddleware<hcl::Body> + 'static,
    {
        self.hcl_deserialize_middleware
            .push(Arc::new(middleware));
        self
    }

    pub fn with_hcl_serialize_middleware<T>(mut self, middleware: T) -> Self
    where
        T: FormatMiddleware<hcl::Body> + 'static,
    {
        self.hcl_serialize_middleware
            .push(Arc::new(middleware));
        self
    }

    pub fn with_json_deserialize_middleware<T>(mut self, middleware: T) -> Self
    where
        T: FormatMiddleware<serde_json::Value> + 'static,
    {
        self.json_deserialize_middleware
            .push(Arc::new(middleware));
        self
    }

    pub fn with_json_serialize_middleware<T>(mut self, middleware: T) -> Self
    where
        T: FormatMiddleware<serde_json::Value> + 'static,
    {
        self.json_serialize_middleware
            .push(Arc::new(middleware));
        self
    }

    pub fn with_yaml_deserialize_middleware<T>(mut self, middleware: T) -> Self
    where
        T: FormatMiddleware<serde_norway::Value> + 'static,
    {
        self.yaml_deserialize_middleware
            .push(Arc::new(middleware));
        self
    }

    pub fn with_yaml_serialize_middleware<T>(mut self, middleware: T) -> Self
    where
        T: FormatMiddleware<serde_norway::Value> + 'static,
    {
        self.yaml_serialize_middleware
            .push(Arc::new(middleware));
        self
    }

    pub fn kind(&self) -> &SpecFormatKind {
        &self.kind
    }

    pub fn extensions(&self) -> &'static [&'static str] {
        match self.kind {
            SpecFormatKind::Hcl => &["acl", "hcl"],
            SpecFormatKind::Yaml => &["yml", "yaml"],
            SpecFormatKind::Json => &["json"],
        }
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ParserError> {
        match path
            .as_ref()
            .extension()
            .and_then(|s| s.to_str())
        {
            Some("acl") | Some("hcl") => Ok(SpecFormat::hcl()),
            Some("yml") | Some("yaml") => Ok(SpecFormat::yaml()),
            Some("json") => Ok(SpecFormat::json()),
            _ => Err(ParserError::UnknownFormat(
                path.as_ref()
                    .to_string_lossy()
                    .into_owned(),
            )),
        }
    }

    pub fn deserialize_string<T>(&self, value: &str) -> Result<T, ParserError>
    where
        T: for<'de> Deserialize<'de>,
    {
        match self.kind() {
            SpecFormatKind::Hcl => {
                let mut body = hcl::from_str::<hcl::Body>(value)?;
                for middleware in &self.hcl_deserialize_middleware {
                    body = middleware.apply(body)?;
                }
                Ok(from_body(body)?)
            }
            SpecFormatKind::Yaml => {
                let mut val = serde_norway::from_str::<serde_norway::Value>(value)?;
                for middleware in &self.yaml_deserialize_middleware {
                    val = middleware.apply(val)?;
                }
                Ok(serde_norway::from_value(val)?)
            }
            SpecFormatKind::Json => {
                let mut val = serde_json::from_str::<serde_json::Value>(value)?;
                for middleware in &self.json_deserialize_middleware {
                    val = middleware.apply(val)?;
                }
                Ok(serde_json::from_value(val)?)
            }
        }
    }

    pub fn serialize_string<T>(&self, value: &T) -> Result<String, ParserError>
    where
        T: Serialize,
    {
        match self.kind() {
            SpecFormatKind::Hcl => {
                let mut body = hcl::Body::from_serializable(value)?;
                for middleware in &self.hcl_serialize_middleware {
                    body = middleware.apply(body)?;
                }
                Ok(hcl::to_string(&body)?)
            }
            SpecFormatKind::Yaml => {
                let mut val = serde_norway::to_value(value)?;
                for middleware in &self.yaml_serialize_middleware {
                    val = middleware.apply(val)?;
                }
                Ok(serde_norway::to_string(&val)?)
            }
            SpecFormatKind::Json => {
                let mut val = serde_json::to_value(value)?;
                for middleware in &self.json_serialize_middleware {
                    val = middleware.apply(val)?;
                }
                Ok(serde_json::to_string_pretty(&val)?)
            }
        }
    }

    pub fn is_hcl(&self) -> bool {
        matches!(self.kind(), SpecFormatKind::Hcl)
    }

    pub fn is_yaml(&self) -> bool {
        matches!(self.kind(), SpecFormatKind::Yaml)
    }

    pub fn is_json(&self) -> bool {
        matches!(self.kind(), SpecFormatKind::Json)
    }
}

impl FileStoredFormat for SpecFormat {
    fn file_extensions(&self) -> &'static [&'static str] {
        self.extensions()
    }
}

impl Format for SpecFormat {
    fn parse(
        &self,
        _uri: Option<&String>,
        text: &str,
    ) -> Result<Map<String, Value>, Box<dyn Error + Send + Sync>> {
        self.deserialize_string(text)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}

impl Debug for SpecFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("SpecFormat")
            .field("kind", &self.kind)
            .finish()
    }
}
