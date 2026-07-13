// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::types::identity::{ModelId, StaticProviderId};

pub const PROVIDER: StaticProviderId = StaticProviderId::new("ollama");

pub const OTEL_PROVIDER_NAME: &str = PROVIDER.as_str();

/// Well-known models available through Ollama. Variants map to the canonical
/// model name string used in Ollama's API. Any model served by Ollama can also
/// be referenced by passing its name directly as a [`ModelId`] string.
pub enum Model {
    Gemma4,
    Granite4,
    Qwen35,
    Qwen3Vl,
    KimiK25,
    KimiK2Thinking,
}

impl Display for Model {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Model::Gemma4 => write!(f, "gemma4"),
            Model::Granite4 => write!(f, "granite4"),
            Model::Qwen35 => write!(f, "qwen-3.5"),
            Model::Qwen3Vl => write!(f, "qwen3-vl"),
            Model::KimiK25 => write!(f, "kimi-k2.5"),
            Model::KimiK2Thinking => write!(f, "kimi-k2-thinking"),
        }
    }
}

impl From<Model> for ModelId {
    fn from(value: Model) -> Self {
        ModelId::from(value.to_string())
    }
}

impl From<Model> for String {
    fn from(value: Model) -> Self {
        value.to_string()
    }
}
