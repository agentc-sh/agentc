// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

use crate::{
    counter::TokenCounter,
    env::{PromptContext, PromptEnv},
    errors::PromptError,
};

/// The role of a message in a prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// A single rendered message with a precomputed token count.
#[derive(Debug, Clone)]
pub struct RenderedMessage {
    pub role: Role,
    pub content: String,
    pub token_count: usize,
}

/// The output of rendering a `PromptTemplate`: an ordered list of rendered messages.
#[derive(Debug, Clone)]
pub struct RenderedPrompt {
    messages: Vec<RenderedMessage>,
}

impl RenderedPrompt {
    pub fn messages(&self) -> &[RenderedMessage] {
        &self.messages
    }

    pub fn total_tokens(&self) -> usize {
        self.messages
            .iter()
            .map(|m| m.token_count)
            .sum()
    }

    pub fn into_messages(self) -> Vec<RenderedMessage> {
        self.messages
    }
}

/// An ordered list of (role, Jinja2 source) pairs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptTemplate {
    parts: Vec<(Role, String)>,
}

impl PromptTemplate {
    pub fn new() -> Self {
        Self { parts: Vec::new() }
    }

    /// Append a message part with the given role and Jinja2 template source.
    pub fn with_part(mut self, role: Role, source: impl Into<String>) -> Self {
        self.parts.push((role, source.into()));
        self
    }

    /// Convenience constructor for a single-system-message template.
    pub fn system(source: impl Into<String>) -> Self {
        Self::new().with_part(Role::System, source)
    }

    /// Consume the template and yield its (role, source) parts in order.
    pub fn into_parts(self) -> impl Iterator<Item = (Role, String)> {
        self.parts.into_iter()
    }

    /// Render all parts against the given env and context, counting tokens with
    /// the supplied counter.
    pub fn render(
        &self,
        env: &PromptEnv,
        context: &PromptContext,
        counter: &dyn TokenCounter,
    ) -> Result<RenderedPrompt, PromptError> {
        Ok(RenderedPrompt {
            messages: self
                .parts
                .iter()
                .map(|(role, source)| {
                    let content = env.render_str(source, context)?;
                    let token_count = counter.count(&content);

                    Ok(RenderedMessage { role: role.clone(), content, token_count })
                })
                .collect::<Result<Vec<_>, PromptError>>()?,
        })
    }
}

impl Default for PromptTemplate {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> From<T> for PromptTemplate
where
    T: AsRef<str>,
{
    fn from(value: T) -> Self {
        Self::system(value.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        counter::CharApproxCounter,
        env::{PromptContext, PromptEnv},
    };
    use serde_json::json;

    fn env() -> PromptEnv {
        PromptEnv::builder().build()
    }
    fn counter() -> CharApproxCounter {
        CharApproxCounter
    }

    #[test]
    fn renders_single_system_message() {
        let tmpl = PromptTemplate::system("You are {{ name }}.");
        let ctx = PromptContext::from_json(json!({ "name": "Aria" }));
        let rendered = tmpl
            .render(&env(), &ctx, &counter())
            .unwrap();

        assert_eq!(rendered.messages().len(), 1);
        assert_eq!(rendered.messages()[0].role, Role::System);
        assert_eq!(rendered.messages()[0].content, "You are Aria.");
    }

    #[test]
    fn renders_multi_part_template_with_correct_roles() {
        let tmpl = PromptTemplate::system("sys: {{ s }}")
            .with_part(Role::User, "usr: {{ u }}")
            .with_part(Role::Assistant, "asst: {{ a }}");
        let ctx = PromptContext::from_json(json!({ "s": "1", "u": "2", "a": "3" }));
        let rendered = tmpl
            .render(&env(), &ctx, &counter())
            .unwrap();

        assert_eq!(rendered.messages().len(), 3);
        assert_eq!(rendered.messages()[0].role, Role::System);
        assert_eq!(rendered.messages()[1].role, Role::User);
        assert_eq!(rendered.messages()[2].role, Role::Assistant);
        assert_eq!(rendered.messages()[2].content, "asst: 3");
    }

    #[test]
    fn token_counts_are_computed_at_render_time() {
        // "aaaaaaaa" = 8 chars, CharApproxCounter gives 8/4 = 2
        let tmpl = PromptTemplate::system("aaaaaaaa");
        let ctx = PromptContext::from_json(json!({}));
        let rendered = tmpl
            .render(&env(), &ctx, &counter())
            .unwrap();

        assert_eq!(rendered.messages()[0].token_count, 2);
    }

    #[test]
    fn total_tokens_sums_all_parts() {
        // "aaaa" = 4 chars = 1 token, "bbbbbbbb" = 8 chars = 2 tokens, total = 3
        let tmpl = PromptTemplate::system("aaaa").with_part(Role::User, "bbbbbbbb");
        let ctx = PromptContext::from_json(json!({}));
        let rendered = tmpl
            .render(&env(), &ctx, &counter())
            .unwrap();

        assert_eq!(rendered.total_tokens(), 3);
    }

    #[test]
    fn strict_mode_fails_on_missing_variable() {
        let tmpl = PromptTemplate::system("Hello {{ missing }}!");
        let ctx = PromptContext::from_json(json!({}));
        assert!(
            tmpl.render(&env(), &ctx, &counter())
                .is_err()
        );
    }

    #[test]
    fn same_template_renders_with_different_contexts() {
        let tmpl = PromptTemplate::system("Hello {{ name }}!");
        let ctx1 = PromptContext::from_json(json!({ "name": "Alice" }));
        let ctx2 = PromptContext::from_json(json!({ "name": "Bob" }));

        let r1 = tmpl
            .render(&env(), &ctx1, &counter())
            .unwrap();
        let r2 = tmpl
            .render(&env(), &ctx2, &counter())
            .unwrap();

        assert_eq!(r1.messages()[0].content, "Hello Alice!");
        assert_eq!(r2.messages()[0].content, "Hello Bob!");
    }

    #[test]
    fn into_messages_yields_all_parts_in_order() {
        let tmpl = PromptTemplate::system("s").with_part(Role::User, "u");
        let ctx = PromptContext::from_json(json!({}));
        let msgs = tmpl
            .render(&env(), &ctx, &counter())
            .unwrap()
            .into_messages();

        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, Role::System);
        assert_eq!(msgs[1].role, Role::User);
    }
}
