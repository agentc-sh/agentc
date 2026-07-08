// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod client;
pub mod config;
pub mod constants;
pub mod factory;
pub mod model;

pub use crate::providers::gemini::{
    client::GeminiClient, config::GeminiConfig, constants::Model, factory::GeminiFactory,
    model::GeminiModel,
};
