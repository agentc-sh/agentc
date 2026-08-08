// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use serde_json::Value;

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
