// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    hash::Hash,
    sync::LazyLock,
};

use regex::Regex;
use serde_json::Value;

use agentc_blocks::types::RuntimeValue;

use crate::manifest::errors::ManifestError;

static PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?P<escape>\$)?\$\{(?P<path>[^}]+)\}").unwrap());

pub trait Interpolate: Sized {
    fn interpolate(self, context: &Value) -> Result<Self, ManifestError>;
}

impl Interpolate for String {
    fn interpolate(self, context: &Value) -> Result<Self, ManifestError> {
        let mut output = String::with_capacity(self.len());
        let mut end = 0;

        for captures in PATTERN.captures_iter(&self) {
            let whole = captures
                .get(0)
                .expect("regex capture has a full match");
            let path = &captures["path"];

            output.push_str(&self[end..whole.start()]);

            if captures.name("escape").is_some() {
                output.push_str(&format!("${{{path}}}"));
            } else {
                let pointer = format!("/{}", path.replace('.', "/"));
                let value = context
                    .pointer(&pointer)
                    .ok_or_else(|| {
                        ManifestError::resolution(format!("unknown reference `${{{path}}}`"))
                    })?;

                match value {
                    Value::String(value) => output.push_str(value),
                    other => output.push_str(&other.to_string()),
                }
            }

            end = whole.end();
        }

        output.push_str(&self[end..]);

        Ok(output)
    }
}

impl<T: Interpolate> Interpolate for Option<T> {
    fn interpolate(self, context: &Value) -> Result<Self, ManifestError> {
        self.map(|value| value.interpolate(context))
            .transpose()
    }
}

impl<T: Interpolate> Interpolate for Vec<T> {
    fn interpolate(self, context: &Value) -> Result<Self, ManifestError> {
        self.into_iter()
            .map(|item| item.interpolate(context))
            .collect()
    }
}

impl<K, V> Interpolate for HashMap<K, V>
where
    K: Interpolate + Hash + Eq,
    V: Interpolate,
{
    fn interpolate(self, context: &Value) -> Result<Self, ManifestError> {
        self.into_iter()
            .map(|(key, value)| Ok((key.interpolate(context)?, value.interpolate(context)?)))
            .collect()
    }
}

impl<K, V> Interpolate for BTreeMap<K, V>
where
    K: Interpolate + Ord,
    V: Interpolate,
{
    fn interpolate(self, context: &Value) -> Result<Self, ManifestError> {
        self.into_iter()
            .map(|(key, value)| Ok((key.interpolate(context)?, value.interpolate(context)?)))
            .collect()
    }
}

impl<T: Interpolate + Ord> Interpolate for BTreeSet<T> {
    fn interpolate(self, context: &Value) -> Result<Self, ManifestError> {
        self.into_iter()
            .map(|value| value.interpolate(context))
            .collect()
    }
}

impl Interpolate for Value {
    fn interpolate(self, context: &Value) -> Result<Self, ManifestError> {
        Ok(match self {
            Value::String(value) => Value::String(value.interpolate(context)?),
            Value::Array(values) => Value::Array(
                values
                    .into_iter()
                    .map(|value| value.interpolate(context))
                    .collect::<Result<_, ManifestError>>()?,
            ),
            Value::Object(values) => Value::Object(
                values
                    .into_iter()
                    .map(|(key, value)| {
                        Ok((key.interpolate(context)?, value.interpolate(context)?))
                    })
                    .collect::<Result<_, ManifestError>>()?,
            ),
            other => other,
        })
    }
}

impl<T: Interpolate> Interpolate for RuntimeValue<T> {
    fn interpolate(self, context: &Value) -> Result<Self, ManifestError> {
        Ok(match self {
            RuntimeValue::Constant(value) => RuntimeValue::Constant(value.interpolate(context)?),
            RuntimeValue::Runtime { env, default, secret } => RuntimeValue::Runtime {
                env,
                default: default.interpolate(context)?,
                secret,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use serde_json::json;

    use agentc_blocks::types::RuntimeValue;

    use crate::manifest::{errors::ManifestError, interpolate::Interpolate};

    #[test]
    fn interpolates_values_and_preserves_escaped_placeholders() {
        let context = json!({ "locals": { "name": "agentc", "count": 3 } });

        assert_eq!(
            "pre-${locals.name}-${locals.count}-$${locals.name}-post"
                .to_owned()
                .interpolate(&context)
                .unwrap(),
            "pre-agentc-3-${locals.name}-post",
        );
    }

    #[test]
    fn missing_reference_is_a_resolution_error() {
        let error = "${locals.missing}"
            .to_owned()
            .interpolate(&json!({ "locals": {} }))
            .unwrap_err();

        assert!(
            matches!(error, ManifestError::Resolution(message) if message.contains("${locals.missing}"))
        );
    }

    #[test]
    fn nested_json_interpolates_keys_and_values() {
        let value = json!({ "${locals.key}": ["${locals.value}", true, 7] });

        assert_eq!(
            value
                .interpolate(&json!({ "locals": { "key": "header", "value": "ready" } }))
                .unwrap(),
            json!({ "header": ["ready", true, 7] }),
        );
    }

    #[test]
    fn optional_and_ordered_containers_propagate_interpolation() {
        let context = json!({ "locals": { "key": "name", "value": "agentc" } });

        assert_eq!(
            None::<String>
                .interpolate(&context)
                .unwrap(),
            None
        );
        assert_eq!(
            Some("${locals.value}".to_owned())
                .interpolate(&context)
                .unwrap(),
            Some("agentc".to_owned()),
        );
        assert_eq!(
            BTreeMap::from([("${locals.key}".to_owned(), "${locals.value}".to_owned())])
                .interpolate(&context)
                .unwrap(),
            BTreeMap::from([("name".to_owned(), "agentc".to_owned())]),
        );
        assert_eq!(
            BTreeSet::from(["${locals.value}".to_owned()])
                .interpolate(&context)
                .unwrap(),
            BTreeSet::from(["agentc".to_owned()]),
        );
    }

    #[test]
    fn runtime_defaults_interpolate_but_environment_names_do_not() {
        let context = json!({ "locals": { "value": "agentc" } });

        assert_eq!(
            RuntimeValue::constant("${locals.value}".to_owned())
                .interpolate(&context)
                .unwrap(),
            RuntimeValue::constant("agentc".to_owned()),
        );
        assert_eq!(
            RuntimeValue::default_runtime("${locals.value}", "${locals.value}".to_owned(),)
                .interpolate(&context)
                .unwrap(),
            RuntimeValue::default_runtime("${locals.value}", "agentc".to_owned()),
        );
        assert_eq!(
            RuntimeValue::<String>::required_runtime("${locals.value}")
                .interpolate(&context)
                .unwrap(),
            RuntimeValue::required_runtime("${locals.value}"),
        );
    }
}
