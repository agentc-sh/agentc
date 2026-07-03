// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use minijinja::{Environment, UndefinedBehavior};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::PromptError;

/// A shared Jinja2 rendering environment.
///
/// Strict undefined variable mode is enabled by default. A render will return
/// an error if the template references a variable not present in the context.
/// Call [`PromptEnvBuilder::lenient`](crate::env::PromptEnvBuilder::lenient) to opt out.
#[derive(Clone)]
pub struct PromptEnv {
    inner: Arc<Environment<'static>>,
}

impl PromptEnv {
    pub fn builder() -> PromptEnvBuilder {
        PromptEnvBuilder::new()
    }

    pub fn render_str(&self, source: &str, context: &PromptContext) -> Result<String, PromptError> {
        self.inner
            .render_str(source, &context.0)
            .map_err(|e| PromptError::Render(e.to_string()))
    }
}

impl Default for PromptEnv {
    fn default() -> Self {
        PromptEnvBuilder::new().build()
    }
}

pub struct PromptEnvBuilder {
    env: Environment<'static>,
}

impl PromptEnvBuilder {
    pub fn new() -> Self {
        let mut env = Environment::new();
        env.set_undefined_behavior(UndefinedBehavior::Strict);
        Self { env }
    }

    /// Switch to lenient undefined variable mode. Missing variables render as
    /// empty strings rather than producing an error.
    pub fn lenient(mut self) -> Self {
        self.env
            .set_undefined_behavior(UndefinedBehavior::Lenient);
        self
    }

    /// Register a custom Jinja2 function available in all templates rendered
    /// by the environment. The function signature follows the same conventions
    /// as [`minijinja::Environment::add_function`](minijinja::Environment::add_function).
    pub fn with_function<N, F, Rv, Args>(mut self, name: N, f: F) -> Self
    where
        N: Into<std::borrow::Cow<'static, str>>,
        F: minijinja::functions::Function<Rv, Args>,
        Rv: minijinja::value::FunctionResult,
        Args: for<'a> minijinja::value::FunctionArgs<'a>,
    {
        self.env.add_function(name, f);
        self
    }

    /// Register a custom Jinja2 filter available in all templates rendered
    /// by the environment. The filter signature follows the same conventions
    /// as [`minijinja::Environment::add_filter`](minijinja::Environment::add_filter).
    pub fn with_filter<N, F, Rv, Args>(mut self, name: N, f: F) -> Self
    where
        N: Into<std::borrow::Cow<'static, str>>,
        F: minijinja::filters::Filter<Rv, Args>,
        Rv: minijinja::value::FunctionResult,
        Args: for<'a> minijinja::value::FunctionArgs<'a>,
    {
        self.env.add_filter(name, f);
        self
    }

    pub fn build(self) -> PromptEnv {
        PromptEnv { inner: Arc::new(self.env) }
    }
}

impl Default for PromptEnvBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The variable context supplied to a single template render call.
///
/// Construct via [`PromptContext::from_json`](crate::env::PromptContext::from_json) (infallible, from a `serde_json::Value`)
/// or [`PromptContext::from_value`](crate::env::PromptContext::from_value) (fallible, from any `Serialize` type).
/// The `context!` macro is the ergonomic way to build one inline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PromptContext(pub(crate) Value);

impl PromptContext {
    /// Infallible construction from a pre-built JSON value.
    /// This is what the `context!` macro uses internally.
    pub fn from_json(value: Value) -> Self {
        Self(value)
    }

    /// Fallible construction from any serializable value.
    pub fn from_value(value: impl Serialize) -> Result<Self, PromptError> {
        serde_json::to_value(value)
            .map(Self)
            .map_err(|e| PromptError::Context(e.to_string()))
    }

    pub fn value(&self) -> &Value {
        &self.0
    }

    /// Merge the keys of a JSON object into this context.
    ///
    /// Only `Value::Object` values are accepted. Any other variant is ignored.
    /// If a key already exists in this context, the incoming value replaces it.
    pub fn merge(&mut self, vars: Value) {
        if let (Value::Object(base), Value::Object(extra)) = (&mut self.0, vars) {
            base.extend(extra);
        }
    }
}

/// Build a `PromptContext` from key-value pairs using natural Rust syntax.
///
/// ```rust,ignore
/// let ctx = context!(agent_name = "Aria", tool_count = 3usize);
/// ```
///
/// Values must be serializable. The macro constructs a JSON object and wraps it
/// in a `PromptContext` via [`PromptContext::from_json`](crate::env::PromptContext::from_json), which is infallible.
#[macro_export]
macro_rules! context {
    ($($key:ident = $value:expr),* $(,)?) => {
        $crate::env::PromptContext::from_json(
            $crate::__private::serde_json::json!({
                $( ::std::stringify!($key): $value ),*
            })
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strict_mode_errors_on_missing_variable() {
        let env = PromptEnv::builder().build();
        let ctx = PromptContext::from_json(json!({}));
        assert!(
            env.render_str("Hello {{ name }}!", &ctx)
                .is_err()
        );
    }

    #[test]
    fn lenient_mode_succeeds_on_missing_variable() {
        let env = PromptEnv::builder().lenient().build();
        let ctx = PromptContext::from_json(json!({}));
        assert!(
            env.render_str("Hello {{ name }}!", &ctx)
                .is_ok()
        );
    }

    #[test]
    fn renders_simple_variable_substitution() {
        let env = PromptEnv::builder().build();
        let ctx = PromptContext::from_json(json!({ "name": "World" }));
        let result = env
            .render_str("Hello {{ name }}!", &ctx)
            .unwrap();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn renders_jinja2_conditional() {
        let env = PromptEnv::builder().build();
        let ctx = PromptContext::from_json(json!({ "enabled": true, "label": "active" }));
        let result = env
            .render_str("{% if enabled %}{{ label }}{% endif %}", &ctx)
            .unwrap();
        assert_eq!(result, "active");
    }

    #[test]
    fn renders_jinja2_loop() {
        let env = PromptEnv::builder().build();
        let ctx = PromptContext::from_json(json!({ "items": ["a", "b", "c"] }));
        let result = env
            .render_str("{% for x in items %}{{ x }}{% endfor %}", &ctx)
            .unwrap();
        assert_eq!(result, "abc");
    }

    #[test]
    fn context_from_value_round_trips() {
        let ctx = PromptContext::from_value(json!({ "key": "value" })).unwrap();
        assert_eq!(ctx.value()["key"], "value");
    }

    #[test]
    fn context_macro_builds_correct_json() {
        let ctx = crate::context!(agent_name = "Aria", count = 3u64);
        assert_eq!(ctx.value()["agent_name"], "Aria");
        assert_eq!(ctx.value()["count"], 3u64);
    }

    #[test]
    fn context_macro_with_no_keys_produces_empty_object() {
        let ctx = crate::context!();
        assert!(
            ctx.value()
                .as_object()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn with_function_registers_callable_in_template() {
        let env = PromptEnv::builder()
            .with_function("shout", |s: String| s.to_uppercase())
            .build();
        let ctx = PromptContext::from_json(json!({ "msg": "hello" }));
        let result = env
            .render_str("{{ shout(msg) }}", &ctx)
            .unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn with_filter_registers_callable_in_template() {
        let env = PromptEnv::builder()
            .with_filter("repeat", |s: String, n: usize| s.repeat(n))
            .build();
        let ctx = PromptContext::from_json(json!({ "word": "ab" }));
        let result = env
            .render_str("{{ word | repeat(3) }}", &ctx)
            .unwrap();
        assert_eq!(result, "ababab");
    }

    #[test]
    fn prompt_env_is_cheap_to_clone() {
        let env = PromptEnv::builder().build();
        let env2 = env.clone();
        let ctx = PromptContext::from_json(json!({ "x": "y" }));
        assert_eq!(
            env.render_str("{{ x }}", &ctx).unwrap(),
            env2.render_str("{{ x }}", &ctx)
                .unwrap(),
        );
    }
}
