// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use serde::{Deserialize, Serialize};

/// A value that can either be a constant or determined at runtime from an environment variable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RuntimeValue<T> {
    Constant(T),
    Runtime {
        env: String,
        #[serde(default)]
        default: Option<T>,
        #[serde(default)]
        secret: bool,
    },
}

impl<T> RuntimeValue<T> {
    pub fn constant(value: T) -> Self {
        Self::Constant(value)
    }

    pub fn required_runtime(env: impl Into<String>) -> Self {
        Self::Runtime {
            env: env.into(),
            default: None,
            secret: false,
        }
    }

    pub fn default_runtime(env: impl Into<String>, default: T) -> Self {
        Self::Runtime {
            env: env.into(),
            default: Some(default),
            secret: false,
        }
    }

    pub fn secret_runtime(env: impl Into<String>) -> Self {
        Self::Runtime {
            env: env.into(),
            default: None,
            secret: true,
        }
    }

    pub fn secret_default_runtime(env: impl Into<String>, default: T) -> Self {
        Self::Runtime {
            env: env.into(),
            default: Some(default),
            secret: true,
        }
    }

    pub fn is_constant(&self) -> bool {
        matches!(self, Self::Constant(_))
    }

    pub fn is_runtime(&self) -> bool {
        matches!(self, Self::Runtime { .. })
    }

    pub fn as_constant(&self) -> Option<&T> {
        match self {
            Self::Constant(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_runtime(&self) -> Option<(&str, Option<&T>, bool)> {
        match self {
            Self::Runtime { env, default, secret } => {
                Some((env.as_str(), default.as_ref(), *secret))
            }
            _ => None,
        }
    }

    pub fn default_value(&self) -> Option<&T> {
        match self {
            Self::Constant(value) => Some(value),
            Self::Runtime { default: Some(d), .. } => Some(d),
            _ => None,
        }
    }

    pub fn render(
        &self,
        constant_expr: impl Fn(&T) -> TokenStream,
        parse_expr: Option<TokenStream>,
    ) -> TokenStream {
        match self {
            Self::Constant(value) => constant_expr(value),
            Self::Runtime { env, default, .. } => {
                let parse = parse_expr.unwrap_or_else(|| quote! { v });

                match default {
                    Some(default) => {
                        let default_tokens = constant_expr(default);

                        quote! {
                            std::env::var(#env)
                                .map(|v| #parse)
                                .unwrap_or_else(|_| #default_tokens)
                        }
                    }
                    None => quote! {
                        std::env::var(#env)
                            .map(|v| #parse)
                            .expect(&format!("Environment variable {} is required but not set", #env))
                    },
                }
            }
        }
    }
}

impl<T: Default> Default for RuntimeValue<T> {
    fn default() -> Self {
        Self::Constant(T::default())
    }
}
