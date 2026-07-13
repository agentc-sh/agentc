// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::types::identity::{ModelId, StaticProviderId};

pub const PROVIDER: StaticProviderId = StaticProviderId::new("xai");

pub const OTEL_PROVIDER_NAME: &str = "x_ai";

pub enum Model {
    Grok3,
    Grok3Fast,
    Grok3Mini,
    Grok3MiniFast,
    Grok4,
}

impl Display for Model {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Grok3 => write!(f, "grok-3"),
            Self::Grok3Fast => write!(f, "grok-3-fast"),
            Self::Grok3Mini => write!(f, "grok-3-mini"),
            Self::Grok3MiniFast => write!(f, "grok-3-mini-fast"),
            Self::Grok4 => write!(f, "grok-4"),
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
