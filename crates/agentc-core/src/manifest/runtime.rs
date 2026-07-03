// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use sanitizer::prelude::*;
use serde::{Deserialize, Serialize};
use validator::Validate;

use agentc_blocks::types::RuntimeValue;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
#[serde(default)]
pub struct ManifestRuntime {
    /// The default tenant ID to use when no tenant ID is provided by the caller.
    pub default_tenant_id: RuntimeValue<String>,
}

impl Default for ManifestRuntime {
    fn default() -> Self {
        Self {
            default_tenant_id: RuntimeValue::constant("default".to_string()),
        }
    }
}
