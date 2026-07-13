// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::types::identity::{ModelId, StaticProviderId};

pub const PROVIDER: StaticProviderId = StaticProviderId::new("gemini");

pub const OTEL_PROVIDER_NAME: &str = "gcp.gemini";

pub enum Model {
    Gemini25Flash,
    Gemini25Pro,
    Gemini20Flash,
    Gemini15Flash,
    Gemini15Pro,
}

impl Display for Model {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Gemini25Flash => write!(f, "gemini-2.5-flash"),
            Self::Gemini25Pro => write!(f, "gemini-2.5-pro"),
            Self::Gemini20Flash => write!(f, "gemini-2.0-flash"),
            Self::Gemini15Flash => write!(f, "gemini-1.5-flash"),
            Self::Gemini15Pro => write!(f, "gemini-1.5-pro"),
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
