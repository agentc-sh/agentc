// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::types::identity::{ModelId, StaticProviderId};

pub const PROVIDER: StaticProviderId = StaticProviderId::new("openai");

pub enum Model {
    Gpt4o,
    Gpt4oMini,
    Gpt4Turbo,
    Gpt4,
    O4Mini,
    O3,
    O3Mini,
    O1,
    O1Mini,
    Gpt41,
    Gpt41Mini,
    Gpt41Nano,
}

impl Display for Model {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Gpt4o => write!(f, "gpt-4o"),
            Self::Gpt4oMini => write!(f, "gpt-4o-mini"),
            Self::Gpt4Turbo => write!(f, "gpt-4-turbo"),
            Self::Gpt4 => write!(f, "gpt-4"),
            Self::O4Mini => write!(f, "o4-mini"),
            Self::O3 => write!(f, "o3"),
            Self::O3Mini => write!(f, "o3-mini"),
            Self::O1 => write!(f, "o1"),
            Self::O1Mini => write!(f, "o1-mini"),
            Self::Gpt41 => write!(f, "gpt-4.1"),
            Self::Gpt41Mini => write!(f, "gpt-4.1-mini"),
            Self::Gpt41Nano => write!(f, "gpt-4.1-nano"),
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
