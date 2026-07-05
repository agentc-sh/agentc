// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod client;
pub mod config;
pub mod constants;
pub mod factory;
pub mod model;

pub use crate::providers::openrouter::{
    client::OpenRouterClient,
    config::OpenRouterConfig,
    factory::OpenRouterFactory,
    model::OpenRouterModel,
};
