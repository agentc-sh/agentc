// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use serde_json::Value;
use std::ops::Deref;

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
pub enum FieldValue {
    /// Baked in at compile time
    Constant { value: Value },
    /// Loaded from an env var at runtime
    Runtime {
        env: String,
        default: Option<Value>,
        secret: bool,
    },
}

impl<T: serde::Serialize> From<&RuntimeValue<T>> for FieldValue {
    fn from(rv: &RuntimeValue<T>) -> Self {
        match rv {
            RuntimeValue::Constant(v) => FieldValue::Constant {
                value: serde_json::to_value(v).unwrap_or(Value::Null),
            },
            RuntimeValue::Runtime { env, default, secret } => FieldValue::Runtime {
                env: env.clone(),
                default: default
                    .as_ref()
                    .map(|d| serde_json::to_value(d).unwrap_or(Value::Null)),
                secret: *secret,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct FieldSpec {
    /// Segments describing where this field lives, e.g. ["provider", "anthropic", "api_key"].
    /// The last segment is the field name; preceding segments define the nested struct path.
    pub path: Vec<String>,
    /// Type-erased value metadata derived from the RuntimeValue.
    pub value: FieldValue,
    /// Token stream for the Rust type of this field, e.g. `|| { quote! { String } }`.
    pub rust_type: fn() -> TokenStream,
}

impl FieldSpec {
    pub fn new<T>(path: &[impl AsRef<str>], value: &RuntimeValue<T>) -> Self
    where
        T: serde::Serialize + IntoTypeTokens,
    {
        Self {
            path: path
                .iter()
                .map(|s| s.as_ref().to_string())
                .collect(),
            value: FieldValue::from(value),
            rust_type: T::type_tokens,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FieldsSpec(pub Vec<FieldSpec>);

impl FieldsSpec {
    pub fn new(fields: Vec<FieldSpec>) -> Self {
        Self(fields)
    }

    pub fn as_inner(&self) -> &[FieldSpec] {
        &self.0
    }

    pub fn as_inner_mut(&mut self) -> &mut [FieldSpec] {
        &mut self.0
    }

    pub fn into_inner(self) -> Vec<FieldSpec> {
        self.0
    }

    pub fn get(&self, path: &[impl AsRef<str>]) -> Option<&FieldSpec> {
        self.as_inner().iter().find(|f| {
            f.path.len() == path.len()
                && f.path
                    .iter()
                    .zip(path.iter())
                    .all(|(a, b)| a == b.as_ref())
        })
    }
}

impl From<Vec<FieldSpec>> for FieldsSpec {
    fn from(fields: Vec<FieldSpec>) -> Self {
        Self::new(fields)
    }
}

impl Deref for FieldsSpec {
    type Target = [FieldSpec];

    fn deref(&self) -> &Self::Target {
        self.as_inner()
    }
}

impl IntoIterator for FieldsSpec {
    type Item = FieldSpec;
    type IntoIter = std::vec::IntoIter<FieldSpec>;

    fn into_iter(self) -> Self::IntoIter {
        self.into_inner().into_iter()
    }
}
