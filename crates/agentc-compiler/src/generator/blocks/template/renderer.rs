// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use minijinja::{Environment, Value as JinjaValue};
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

use crate::generator::{errors::GeneratorError, extension::ExtensionRegistry};

/// Renders Jinja2 template strings against a serialized context value.
///
/// The main context value is accessible in templates as `{{ ctx.* }}`. Any
/// extra variables supplied at construction are available as additional
/// top-level template variables alongside `ctx`.
///
/// Extension point values are accessible via ``{{ extension("point_name") }}``.
pub(crate) struct TemplateRenderer {
    jinja: JinjaValue,
    extra_vars: BTreeMap<String, JsonValue>,
}

impl TemplateRenderer {
    pub(crate) fn new<T: Serialize>(data: &T) -> Self {
        Self {
            jinja: JinjaValue::from_serialize(data),
            extra_vars: BTreeMap::new(),
        }
    }

    pub(crate) fn with_vars<T: Serialize>(
        data: &T,
        extra_vars: BTreeMap<String, JsonValue>,
    ) -> Self {
        Self {
            jinja: JinjaValue::from_serialize(data),
            extra_vars,
        }
    }

    pub(crate) fn render(
        &self,
        block_id: &str,
        template_name: &str,
        template_src: &str,
        registry: &ExtensionRegistry,
    ) -> Result<String, GeneratorError> {
        let registry = registry.clone();
        let mut env = Environment::new();

        env.add_function("extension", move |name: String| -> String {
            registry
                .get(&name)
                .unwrap_or_default()
                .to_string()
        });

        env.add_template(template_name, template_src)
            .map_err(|e| GeneratorError::RenderFailed {
                block_id: block_id.to_string(),
                source: e,
            })?;

        // Build the template context with `ctx` as the primary key, merging
        // any extra variables as additional top-level keys.
        let ctx = JinjaValue::from(
            std::iter::once(("ctx".to_string(), JinjaValue::from_serialize(&self.jinja)))
                .chain(
                    self.extra_vars
                        .iter()
                        .map(|(k, v)| (k.clone(), JinjaValue::from_serialize(v))),
                )
                .collect::<BTreeMap<_, _>>(),
        );

        env.get_template(template_name)
            .unwrap()
            .render(ctx)
            .map_err(|e| GeneratorError::RenderFailed {
                block_id: block_id.to_string(),
                source: e,
            })
    }
}
