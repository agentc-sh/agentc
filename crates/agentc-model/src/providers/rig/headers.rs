// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, HashMap};

use rig_core::http_client::HeaderMap;
use serde::{Deserialize, Serialize};

#[cfg(feature = "cel")]
use crate::providers::rig::cel::CelExpression;
use crate::{errors::ModelError, types::context::InvocationContext};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "syntax", content = "source", rename_all = "snake_case")]
pub enum HeaderSource {
    Literal(String),
    #[cfg(feature = "cel")]
    Cel(CelExpression),
}

impl HeaderSource {
    pub fn literal(source: impl Into<String>) -> Self {
        Self::Literal(source.into())
    }

    #[cfg(feature = "cel")]
    pub fn cel(source: impl Into<String>) -> Result<Self, ModelError> {
        Ok(Self::Cel(CelExpression::new(source)?))
    }

    #[cfg_attr(not(feature = "cel"), allow(unused_variables))]
    pub fn evaluate(&self, context: &InvocationContext) -> Result<String, ModelError> {
        match self {
            Self::Literal(source) => Ok(source.clone()),
            #[cfg(feature = "cel")]
            Self::Cel(expression) => expression.evaluate(context),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Headers(BTreeMap<String, HeaderSource>);

impl Headers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, name: impl Into<String>, source: HeaderSource) -> Self {
        self.0.insert(name.into(), source);
        self
    }

    pub fn resolve(&self, context: &InvocationContext) -> Result<HeaderMap, ModelError> {
        HeaderMap::try_from(
            &self
                .0
                .iter()
                .map(|(name, source)| Ok((name.clone(), source.evaluate(context)?)))
                .collect::<Result<HashMap<_, _>, ModelError>>()?,
        )
        .map_err(|e| ModelError::invalid_request(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn literal_evaluates_verbatim() {
        assert_eq!(
            HeaderSource::literal("tenant_id")
                .evaluate(&InvocationContext::new().with_attribute("tenant_id", "acme"))
                .unwrap(),
            "tenant_id"
        );
    }

    #[test]
    fn literal_round_trips() {
        assert_eq!(
            serde_json::to_value(HeaderSource::literal("platform")).unwrap(),
            json!({ "syntax": "literal", "source": "platform" })
        );
        assert!(matches!(
            serde_json::from_value::<HeaderSource>(
                json!({ "syntax": "literal", "source": "platform" })
            )
            .unwrap(),
            HeaderSource::Literal(source) if source == "platform"
        ));
    }

    #[cfg(not(feature = "cel"))]
    #[test]
    fn cel_is_unknown_without_the_feature() {
        assert!(
            serde_json::from_value::<HeaderSource>(
                json!({ "syntax": "cel", "source": "tenant_id" })
            )
            .is_err()
        );
    }

    #[test]
    fn resolves_every_header() {
        let headers = Headers::new()
            .with("X-Team", HeaderSource::literal("platform"))
            .resolve(&InvocationContext::new())
            .unwrap();

        assert_eq!(headers["x-team"], "platform");
    }

    #[test]
    fn empty_headers_resolve_to_empty_map() {
        assert!(
            Headers::new()
                .resolve(&InvocationContext::new())
                .unwrap()
                .is_empty()
        );
    }
}
