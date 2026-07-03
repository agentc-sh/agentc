// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use regex::{Captures, Regex};
use serde_json::Value;
use std::{collections::HashMap, hash::Hash, sync::LazyLock};

use agentc_blocks::types::RuntimeValue;

static PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?P<escape>\$)?\$\{(?P<path>[^}]+)\}").unwrap());

/// A trait for interpolating strings with values from a JSON context.
pub trait Interpolate {
    fn interpolate(self, context: &Value) -> Self;
}

impl Interpolate for String {
    fn interpolate(self, context: &Value) -> Self {
        PATTERN
            .replace_all(self.as_ref(), |caps: &Captures| {
                if caps.name("escape").is_some() {
                    return format!("${{{}}}", &caps["path"]);
                }

                let path = &caps["path"];
                let pointer = format!("/{}", path.replace('.', "/"));

                context
                    .pointer(&pointer)
                    .map(|value| match value {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .unwrap_or_default()
            })
            .into_owned()
    }
}

impl<T: Interpolate> Interpolate for Vec<T> {
    fn interpolate(self, context: &Value) -> Self {
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
    fn interpolate(self, context: &Value) -> Self {
        self.into_iter()
            .map(|(k, v)| (k.interpolate(context), v.interpolate(context)))
            .collect()
    }
}

impl Interpolate for Value {
    fn interpolate(self, context: &Value) -> Self {
        match self {
            Value::String(s) => Value::String(s.interpolate(context)),
            Value::Array(arr) => Value::Array(
                arr.into_iter()
                    .map(|v| v.interpolate(context))
                    .collect(),
            ),
            Value::Object(obj) => Value::Object(
                obj.into_iter()
                    .map(|(k, v)| (k.interpolate(context), v.interpolate(context)))
                    .collect(),
            ),
            other => other,
        }
    }
}

impl<T: Interpolate> Interpolate for RuntimeValue<T> {
    fn interpolate(self, context: &Value) -> Self {
        match self {
            RuntimeValue::Constant(value) => RuntimeValue::Constant(value.interpolate(context)),
            RuntimeValue::Runtime { env, default, secret } => RuntimeValue::Runtime {
                env,
                default: default.map(|d| d.interpolate(context)),
                secret,
            },
        }
    }
}
