// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use minijinja::{AutoEscape, Environment, default_auto_escape_callback, escape_formatter};
use serde::Serialize;
use std::{collections::BTreeMap, fmt::Write};

use crate::generator::{errors::GeneratorError, extension::ExtensionRegistry};

pub struct TomlEscaped<'a> {
    value: &'a str,
}

impl<'a> TomlEscaped<'a> {
    pub fn new(value: &'a str) -> Self {
        Self { value }
    }
}

impl std::fmt::Display for TomlEscaped<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for character in self.value.chars() {
            match character {
                '"' => formatter.write_str("\\\"")?,
                '\\' => formatter.write_str("\\\\")?,
                '\u{8}' => formatter.write_str("\\b")?,
                '\t' => formatter.write_str("\\t")?,
                '\n' => formatter.write_str("\\n")?,
                '\u{c}' => formatter.write_str("\\f")?,
                '\r' => formatter.write_str("\\r")?,
                character if character.is_control() => {
                    write!(formatter, "\\u{:04X}", u32::from(character))?
                }
                character => formatter.write_char(character)?,
            }
        }

        Ok(())
    }
}

/// Renders Jinja2 template strings against a serialized context value.
///
/// The main context value is accessible in templates as `{{ ctx.* }}`. Any
/// extra variables supplied at construction are available as additional
/// top-level template variables alongside `ctx`.
///
/// Extension point values are accessible via ``{{ extension("point_name") }}``.
pub(crate) struct TemplateRenderer {
    jinja: minijinja::Value,
    extra_vars: BTreeMap<String, serde_json::Value>,
}

impl TemplateRenderer {
    pub(crate) fn new<T: Serialize>(data: &T) -> Self {
        Self {
            jinja: minijinja::Value::from_serialize(data),
            extra_vars: BTreeMap::new(),
        }
    }

    pub(crate) fn with_vars(mut self, vars: BTreeMap<String, serde_json::Value>) -> Self {
        self.extra_vars = vars;
        self
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

        env.set_auto_escape_callback(|name| match name.rsplit('.').next() {
            Some("toml") => AutoEscape::Custom("toml"),
            _ => default_auto_escape_callback(name),
        });

        env.set_formatter(|out, state, value| match (state.auto_escape(), value.as_str()) {
            (AutoEscape::Custom("toml"), Some(text)) if !value.is_safe() => {
                write!(out, "{}", TomlEscaped::new(text)).map_err(minijinja::Error::from)
            }
            (AutoEscape::Custom("toml"), _) => {
                write!(out, "{value}").map_err(minijinja::Error::from)
            }
            _ => escape_formatter(out, state, value),
        });

        env.add_function("extension", move |name: String| -> minijinja::Value {
            minijinja::Value::from_safe_string(
                registry
                    .get(&name)
                    .unwrap_or_default()
                    .to_string(),
            )
        });

        env.add_template(template_name, template_src)
            .map_err(|e| GeneratorError::RenderFailed {
                block_id: block_id.to_string(),
                source: e,
            })?;

        // Build the template context with `ctx` as the primary key, merging
        // any extra variables as additional top-level keys.
        let ctx = minijinja::Value::from(
            std::iter::once(("ctx".to_string(), minijinja::Value::from_serialize(&self.jinja)))
                .chain(
                    self.extra_vars
                        .iter()
                        .map(|(k, v)| (k.clone(), minijinja::Value::from_serialize(v))),
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::generator::{
        blocks::template::renderer::{TemplateRenderer, TomlEscaped},
        extension::ExtensionRegistry,
    };

    #[test]
    fn plain_string_is_unchanged() {
        assert_eq!(TomlEscaped::new("0.2").to_string(), "0.2");
    }

    #[test]
    fn quote_and_backslash_are_escaped() {
        assert_eq!(
            TomlEscaped::new("C:\\deps\\\"foo\"").to_string(),
            "C:\\\\deps\\\\\\\"foo\\\"",
        );
    }

    #[test]
    fn control_characters_are_escaped() {
        assert_eq!(
            TomlEscaped::new("a\nb\u{1f}\u{7f}").to_string(),
            "a\\nb\\u001F\\u007F",
        );
    }

    #[test]
    fn toml_file_strings_are_escaped() {
        assert_eq!(
            TemplateRenderer::new(&json!({ "description": "The \"fast\" agent" }))
                .render(
                    "block",
                    "Cargo.toml",
                    "description = \"{{ ctx.description }}\"",
                    &ExtensionRegistry::empty(),
                )
                .unwrap(),
            "description = \"The \\\"fast\\\" agent\"",
        );
    }

    #[test]
    fn toml_file_safe_strings_are_unchanged() {
        assert_eq!(
            TemplateRenderer::new(&json!({ "features": "\"a\", \"b\"" }))
                .render(
                    "block",
                    "Cargo.toml",
                    "features = [{{ ctx.features | safe }}]",
                    &ExtensionRegistry::empty(),
                )
                .unwrap(),
            "features = [\"a\", \"b\"]",
        );
    }

    #[test]
    fn other_file_strings_are_unchanged() {
        assert_eq!(
            TemplateRenderer::new(&json!({ "description": "The \"fast\" agent" }))
                .render(
                    "block",
                    "src/main.rs",
                    "{{ ctx.description }}",
                    &ExtensionRegistry::empty(),
                )
                .unwrap(),
            "The \"fast\" agent",
        );
    }
}
