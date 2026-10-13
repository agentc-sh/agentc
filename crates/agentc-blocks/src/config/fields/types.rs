// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use serde::Serialize;
use serde_json::{Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    iter::once,
};

use crate::types::RuntimeValue;

pub trait IntoTypeTokens {
    fn type_tokens() -> TokenStream;
}

impl IntoTypeTokens for String {
    fn type_tokens() -> TokenStream {
        quote! { String }
    }
}

impl IntoTypeTokens for &str {
    fn type_tokens() -> TokenStream {
        quote! { &str }
    }
}

impl IntoTypeTokens for bool {
    fn type_tokens() -> TokenStream {
        quote! { bool }
    }
}

impl IntoTypeTokens for u64 {
    fn type_tokens() -> TokenStream {
        quote! { u64 }
    }
}

impl IntoTypeTokens for u32 {
    fn type_tokens() -> TokenStream {
        quote! { u32 }
    }
}

impl IntoTypeTokens for u16 {
    fn type_tokens() -> TokenStream {
        quote! { u16 }
    }
}

impl IntoTypeTokens for u8 {
    fn type_tokens() -> TokenStream {
        quote! { u8 }
    }
}

impl IntoTypeTokens for usize {
    fn type_tokens() -> TokenStream {
        quote! { usize }
    }
}

impl IntoTypeTokens for isize {
    fn type_tokens() -> TokenStream {
        quote! { isize }
    }
}

impl IntoTypeTokens for i64 {
    fn type_tokens() -> TokenStream {
        quote! { i64 }
    }
}

impl IntoTypeTokens for i32 {
    fn type_tokens() -> TokenStream {
        quote! { i32 }
    }
}

impl IntoTypeTokens for f64 {
    fn type_tokens() -> TokenStream {
        quote! { f64 }
    }
}

impl IntoTypeTokens for f32 {
    fn type_tokens() -> TokenStream {
        quote! { f32 }
    }
}

impl IntoTypeTokens for Value {
    fn type_tokens() -> TokenStream {
        quote! { serde_json::Value }
    }
}

impl<T> IntoTypeTokens for Vec<T>
where
    T: IntoTypeTokens,
{
    fn type_tokens() -> TokenStream {
        let t = T::type_tokens();
        quote! { Vec<#t> }
    }
}

impl<T> IntoTypeTokens for Option<T>
where
    T: IntoTypeTokens,
{
    fn type_tokens() -> TokenStream {
        let t = T::type_tokens();
        quote! { Option<#t> }
    }
}

#[derive(Debug, Clone)]
pub enum FieldSegment {
    Key(String),
    Index(usize),
}

impl ToTokens for FieldSegment {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Key(key) => key.to_tokens(tokens),
            Self::Index(index) => index.to_tokens(tokens),
        }
    }
}

impl From<&str> for FieldSegment {
    fn from(key: &str) -> Self {
        Self::Key(key.to_string())
    }
}

impl From<String> for FieldSegment {
    fn from(key: String) -> Self {
        Self::Key(key)
    }
}

#[derive(Default)]
pub struct FieldTokens {
    loader: Vec<TokenStream>,
    mapper: Vec<TokenStream>,
}

impl FieldTokens {
    pub fn push_loader(&mut self, tokens: TokenStream) {
        self.loader.push(tokens);
    }

    pub fn push_mapper(&mut self, tokens: TokenStream) {
        self.mapper.push(tokens);
    }

    pub fn extend(&mut self, other: FieldTokens) {
        self.loader.extend(other.loader);
        self.mapper.extend(other.mapper);
    }

    pub fn loader(&self) -> &[TokenStream] {
        &self.loader
    }

    pub fn mapper(&self) -> &[TokenStream] {
        &self.mapper
    }
}

#[derive(Debug, Clone)]
pub enum FieldDeclaration {
    Constant(Value),
    Default(Value),
    Seal,
}

impl FieldDeclaration {
    fn json(&self) -> TokenStream {
        match self {
            Self::Constant(value) | Self::Default(value) => {
                let value = value
                    .to_string()
                    .parse::<TokenStream>()
                    .unwrap();

                quote! { serde_json::json!(#value) }
            }
            Self::Seal => TokenStream::new(),
        }
    }

    pub fn call(&self, path: &[FieldSegment]) -> TokenStream {
        let json = self.json();

        match self {
            Self::Constant(_) => quote! { .constant(path![#(#path),*], #json) },
            Self::Default(_) => quote! { .default(path![#(#path),*], #json) },
            Self::Seal => quote! { .seal(path![#(#path),*]) },
        }
    }

    pub fn attribute(&self) -> TokenStream {
        let json = self.json();

        match self {
            Self::Constant(_) => quote! { #[config(constant = #json)] },
            Self::Default(_) => quote! { #[config(default = #json)] },
            Self::Seal => quote! { #[config(seal)] },
        }
    }
}

#[derive(Debug, Clone)]
pub enum FieldValue {
    /// Baked in at compile time
    Constant {
        value: Value,
    },
    /// Loaded from an env var at runtime
    Runtime {
        env: String,
        default: Option<Box<FieldValue>>,
        secret: bool,
    },
    Map {
        entries: BTreeMap<String, FieldValue>,
    },
    Sequence {
        items: Vec<FieldValue>,
    },
}

impl FieldValue {
    pub fn constant(value: impl Serialize) -> Self {
        Self::Constant {
            value: serde_json::to_value(value).unwrap_or(Value::Null),
        }
    }

    fn json(&self) -> Option<Value> {
        match self {
            Self::Constant { value } => Some(value.clone()),
            Self::Runtime { .. } => None,
            Self::Map { entries } => entries
                .iter()
                .map(|(key, child)| {
                    child
                        .json()
                        .map(|value| (key.clone(), value))
                })
                .collect::<Option<Map<_, _>>>()
                .map(Value::Object),
            Self::Sequence { items } => items
                .iter()
                .map(Self::json)
                .collect::<Option<Vec<_>>>()
                .map(Value::Array),
        }
    }

    fn children(&self) -> Vec<(FieldSegment, &FieldValue)> {
        match self {
            Self::Map { entries } => entries
                .iter()
                .map(|(key, child)| (FieldSegment::Key(key.clone()), child))
                .collect(),
            Self::Sequence { items } => items
                .iter()
                .enumerate()
                .map(|(index, child)| (FieldSegment::Index(index), child))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn declaration(&self, fixed: bool) -> Option<FieldDeclaration> {
        if let Self::Runtime { default, .. } = self {
            return default
                .as_ref()
                .and_then(|default| default.declaration(false));
        }

        match (self.json(), fixed) {
            (Some(value), true) => Some(FieldDeclaration::Constant(value)),
            (Some(value), false) => Some(FieldDeclaration::Default(value)),
            (None, true) => Some(FieldDeclaration::Seal),
            (None, false) => None,
        }
    }

    fn extend_inner_tokens(&self, path: &[FieldSegment], fixed: bool, tokens: &mut FieldTokens) {
        if let Self::Runtime { env, default, .. } = self {
            tokens.push_mapper(quote! { .alias(path![#(#path),*], #env) });

            if let Some(default) = default {
                default.extend_inner_tokens(path, false, tokens);
            }

            return;
        }

        if self.json().is_some() {
            return;
        }

        for (segment, child) in self.children() {
            child.extend_tokens(
                &path
                    .iter()
                    .cloned()
                    .chain(once(segment))
                    .collect::<Vec<_>>(),
                fixed,
                tokens,
            );
        }
    }

    fn extend_tokens(&self, path: &[FieldSegment], fixed: bool, tokens: &mut FieldTokens) {
        if let Some(declaration) = self.declaration(fixed) {
            tokens.push_loader(declaration.call(path));
        }

        self.extend_inner_tokens(path, fixed, tokens);
    }

    pub fn is_secret(&self) -> bool {
        matches!(self, Self::Runtime { secret: true, .. })
    }

    pub fn attribute(&self) -> Option<TokenStream> {
        self.declaration(true)
            .map(|declaration| declaration.attribute())
    }

    pub fn inner_tokens(&self, path: &[FieldSegment]) -> FieldTokens {
        let mut tokens = FieldTokens::default();

        self.extend_inner_tokens(path, true, &mut tokens);

        tokens
    }

    pub fn tokens(&self, path: &[FieldSegment]) -> FieldTokens {
        let mut tokens = FieldTokens::default();

        self.extend_tokens(path, true, &mut tokens);

        tokens
    }
}

pub trait AsFieldValue {
    fn as_field_value(&self) -> FieldValue;
}

impl AsFieldValue for String {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::constant(self)
    }
}

impl AsFieldValue for bool {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::constant(self)
    }
}

impl AsFieldValue for u64 {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::constant(self)
    }
}

impl AsFieldValue for u32 {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::constant(self)
    }
}

impl AsFieldValue for u16 {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::constant(self)
    }
}

impl AsFieldValue for u8 {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::constant(self)
    }
}

impl AsFieldValue for usize {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::constant(self)
    }
}

impl AsFieldValue for isize {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::constant(self)
    }
}

impl AsFieldValue for i64 {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::constant(self)
    }
}

impl AsFieldValue for i32 {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::constant(self)
    }
}

impl AsFieldValue for f64 {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::constant(self)
    }
}

impl AsFieldValue for f32 {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::constant(self)
    }
}

impl AsFieldValue for Value {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::constant(self)
    }
}

impl<T: AsFieldValue> AsFieldValue for Option<T> {
    fn as_field_value(&self) -> FieldValue {
        match self {
            Some(value) => value.as_field_value(),
            None => FieldValue::constant(Value::Null),
        }
    }
}

impl<T: AsFieldValue> AsFieldValue for Vec<T> {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::Sequence {
            items: self
                .iter()
                .map(AsFieldValue::as_field_value)
                .collect(),
        }
    }
}

impl<T: AsFieldValue> AsFieldValue for BTreeSet<T> {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::Sequence {
            items: self
                .iter()
                .map(AsFieldValue::as_field_value)
                .collect(),
        }
    }
}

impl<T: AsFieldValue> AsFieldValue for BTreeMap<String, T> {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::Map {
            entries: self
                .iter()
                .map(|(key, value)| (key.clone(), value.as_field_value()))
                .collect(),
        }
    }
}

impl<T: AsFieldValue> AsFieldValue for HashMap<String, T> {
    fn as_field_value(&self) -> FieldValue {
        FieldValue::Map {
            entries: self
                .iter()
                .map(|(key, value)| (key.clone(), value.as_field_value()))
                .collect(),
        }
    }
}

impl<T: AsFieldValue> AsFieldValue for RuntimeValue<T> {
    fn as_field_value(&self) -> FieldValue {
        match self {
            RuntimeValue::Constant(value) => value.as_field_value(),
            RuntimeValue::Runtime { env, default, secret } => FieldValue::Runtime {
                env: env.clone(),
                default: default
                    .as_ref()
                    .map(|default| Box::new(default.as_field_value())),
                secret: *secret,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Emitted {
        loader: Vec<String>,
        mapper: Vec<String>,
    }

    impl From<FieldTokens> for Emitted {
        fn from(tokens: FieldTokens) -> Self {
            Self {
                loader: tokens
                    .loader()
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                mapper: tokens
                    .mapper()
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            }
        }
    }

    #[test]
    fn a_subtree_without_runtime_values_is_one_constant() {
        let value = FieldValue::Map {
            entries: BTreeMap::from([
                ("a".to_string(), FieldValue::constant(1)),
                ("b".to_string(), FieldValue::Sequence { items: vec![FieldValue::constant("x")] }),
            ]),
        };

        let emitted = Emitted::from(value.tokens(&["n".into()]));

        assert_eq!(
            emitted.loader,
            vec![
                quote! { .constant(path!["n"], serde_json::json!({"a":1,"b":["x"]})) }.to_string()
            ],
        );
        assert!(emitted.mapper.is_empty());
    }

    #[test]
    fn a_runtime_value_aliases_its_path_and_defaults_it() {
        let emitted = Emitted::from(
            FieldValue::Runtime {
                env: "PORT".to_string(),
                default: Some(Box::new(FieldValue::constant(8080))),
                secret: false,
            }
            .tokens(&["server".into(), "port".into()]),
        );

        assert_eq!(
            emitted.loader,
            vec![quote! { .default(path!["server", "port"], serde_json::json!(8080)) }.to_string()],
        );
        assert_eq!(
            emitted.mapper,
            vec![quote! { .alias(path!["server", "port"], "PORT") }.to_string()],
        );
    }

    #[test]
    fn a_runtime_value_without_a_default_only_aliases_its_path() {
        let emitted = Emitted::from(
            FieldValue::Runtime {
                env: "KEY".to_string(),
                default: None,
                secret: false,
            }
            .tokens(&["key".into()]),
        );

        assert!(emitted.loader.is_empty());
        assert_eq!(emitted.mapper, vec![quote! { .alias(path!["key"], "KEY") }.to_string()]);
    }

    #[test]
    fn a_fixed_map_with_a_runtime_value_seals_then_declares_each_child() {
        let value = FieldValue::Map {
            entries: BTreeMap::from([
                ("a".to_string(), FieldValue::constant(1)),
                (
                    "b".to_string(),
                    FieldValue::Runtime {
                        env: "B".to_string(),
                        default: None,
                        secret: false,
                    },
                ),
            ]),
        };

        let emitted = Emitted::from(value.tokens(&["n".into()]));

        assert_eq!(
            emitted.loader,
            vec![
                quote! { .seal(path!["n"]) }.to_string(),
                quote! { .constant(path!["n", "a"], serde_json::json!(1)) }.to_string(),
            ],
        );
        assert_eq!(emitted.mapper, vec![quote! { .alias(path!["n", "b"], "B") }.to_string()]);
    }

    #[test]
    fn a_fixed_sequence_with_a_runtime_value_declares_each_item_by_index() {
        let value = FieldValue::Sequence {
            items: vec![
                FieldValue::Runtime {
                    env: "X0".to_string(),
                    default: None,
                    secret: false,
                },
                FieldValue::constant("y"),
            ],
        };

        let emitted = Emitted::from(value.tokens(&["s".into()]));

        assert_eq!(
            emitted.loader,
            vec![
                quote! { .seal(path!["s"]) }.to_string(),
                quote! { .constant(path!["s", 1usize], serde_json::json!("y")) }.to_string(),
            ],
        );
        assert_eq!(emitted.mapper, vec![quote! { .alias(path!["s", 0usize], "X0") }.to_string()]);
    }

    #[test]
    fn a_runtime_default_declares_defaults_and_never_seals() {
        let value = FieldValue::Runtime {
            env: "N".to_string(),
            default: Some(Box::new(FieldValue::Map {
                entries: BTreeMap::from([
                    ("a".to_string(), FieldValue::constant(1)),
                    (
                        "b".to_string(),
                        FieldValue::Runtime {
                            env: "B".to_string(),
                            default: None,
                            secret: false,
                        },
                    ),
                ]),
            })),
            secret: false,
        };

        let emitted = Emitted::from(value.tokens(&["n".into()]));

        assert_eq!(
            emitted.loader,
            vec![quote! { .default(path!["n", "a"], serde_json::json!(1)) }.to_string()],
        );
        assert_eq!(
            emitted.mapper,
            vec![
                quote! { .alias(path!["n"], "N") }.to_string(),
                quote! { .alias(path!["n", "b"], "B") }.to_string(),
            ],
        );
    }

    #[test]
    fn attribute_and_inner_tokens_split_what_tokens_emits() {
        let value = FieldValue::Map {
            entries: BTreeMap::from([
                ("a".to_string(), FieldValue::constant(1)),
                (
                    "b".to_string(),
                    FieldValue::Runtime {
                        env: "B".to_string(),
                        default: None,
                        secret: false,
                    },
                ),
            ]),
        };

        let inner = Emitted::from(value.inner_tokens(&["n".into()]));

        assert_eq!(
            value
                .attribute()
                .map(|attribute| attribute.to_string()),
            Some(quote! { #[config(seal)] }.to_string()),
        );
        assert_eq!(
            inner.loader,
            vec![quote! { .constant(path!["n", "a"], serde_json::json!(1)) }.to_string()],
        );
        assert_eq!(inner.mapper, vec![quote! { .alias(path!["n", "b"], "B") }.to_string()]);
    }

    #[test]
    fn attribute_renders_each_declaration() {
        assert_eq!(
            FieldValue::constant("claude")
                .attribute()
                .map(|attribute| attribute.to_string()),
            Some(quote! { #[config(constant = serde_json::json!("claude"))] }.to_string()),
        );
        assert_eq!(
            FieldValue::Runtime {
                env: "PORT".to_string(),
                default: Some(Box::new(FieldValue::constant(8080))),
                secret: false,
            }
            .attribute()
            .map(|attribute| attribute.to_string()),
            Some(quote! { #[config(default = serde_json::json!(8080))] }.to_string()),
        );
        assert!(
            FieldValue::Runtime {
                env: "KEY".to_string(),
                default: None,
                secret: false,
            }
            .attribute()
            .is_none()
        );
    }

    #[test]
    fn is_secret_holds_only_for_a_secret_runtime_value() {
        let secret = FieldValue::Runtime {
            env: "KEY".to_string(),
            default: None,
            secret: true,
        };

        assert!(secret.is_secret());
        assert!(
            !FieldValue::Runtime {
                env: "KEY".to_string(),
                default: None,
                secret: false,
            }
            .is_secret()
        );
        assert!(!FieldValue::constant("x").is_secret());
    }

    #[test]
    fn field_tokens_extend_appends_both_lists() {
        let mut tokens = FieldValue::Runtime {
            env: "A".to_string(),
            default: Some(Box::new(FieldValue::constant(1))),
            secret: false,
        }
        .tokens(&["a".into()]);

        tokens.extend(
            FieldValue::Runtime {
                env: "B".to_string(),
                default: Some(Box::new(FieldValue::constant(2))),
                secret: false,
            }
            .tokens(&["b".into()]),
        );

        let emitted = Emitted::from(tokens);

        assert_eq!(emitted.loader.len(), 2);
        assert_eq!(emitted.mapper.len(), 2);
    }

    #[test]
    fn a_runtime_value_converts_its_default_as_a_tree() {
        let value =
            RuntimeValue::secret_default_runtime("TAGS", vec!["a".to_string()]).as_field_value();

        let emitted = Emitted::from(value.tokens(&["tags".into()]));

        assert!(value.is_secret());
        assert_eq!(
            emitted.loader,
            vec![quote! { .default(path!["tags"], serde_json::json!(["a"])) }.to_string()],
        );
        assert_eq!(emitted.mapper, vec![quote! { .alias(path!["tags"], "TAGS") }.to_string()]);
    }

    #[test]
    fn a_constant_runtime_value_is_its_value() {
        let emitted = Emitted::from(
            RuntimeValue::constant(8080u16)
                .as_field_value()
                .tokens(&["port".into()]),
        );

        assert_eq!(
            emitted.loader,
            vec![quote! { .constant(path!["port"], serde_json::json!(8080)) }.to_string()],
        );
        assert!(emitted.mapper.is_empty());
    }

    #[test]
    fn none_is_a_null_constant() {
        let emitted = Emitted::from(
            None::<String>
                .as_field_value()
                .tokens(&["key".into()]),
        );

        assert_eq!(
            emitted.loader,
            vec![quote! { .constant(path!["key"], serde_json::json!(null)) }.to_string()],
        );
    }

    #[test]
    fn maps_convert_entry_by_entry() {
        let emitted = Emitted::from(
            BTreeMap::from([
                ("a".to_string(), RuntimeValue::constant("x".to_string())),
                ("b".to_string(), RuntimeValue::required_runtime("B")),
            ])
            .as_field_value()
            .tokens(&["headers".into()]),
        );

        assert_eq!(
            emitted.loader,
            vec![
                quote! { .seal(path!["headers"]) }.to_string(),
                quote! { .constant(path!["headers", "a"], serde_json::json!("x")) }.to_string(),
            ],
        );
        assert_eq!(emitted.mapper, vec![quote! { .alias(path!["headers", "b"], "B") }.to_string()]);
        assert_eq!(
            Emitted::from(
                HashMap::from([("a".to_string(), "x".to_string())])
                    .as_field_value()
                    .tokens(&["env".into()]),
            )
            .loader,
            vec![quote! { .constant(path!["env"], serde_json::json!({"a": "x"})) }.to_string()],
        );
    }

    #[test]
    fn sets_convert_to_sequences_in_order() {
        let emitted = Emitted::from(
            BTreeSet::from(["b".to_string(), "a".to_string()])
                .as_field_value()
                .tokens(&["methods".into()]),
        );

        assert_eq!(
            emitted.loader,
            vec![quote! { .constant(path!["methods"], serde_json::json!(["a", "b"])) }.to_string()],
        );
    }
}
