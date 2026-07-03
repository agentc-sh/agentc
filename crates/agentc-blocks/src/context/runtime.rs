// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

use crate::types::RuntimeValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextRuntime {
    /// The default tenant ID to use when no tenant ID is provided by the caller.
    pub default_tenant_id: RuntimeValue<String>,
}
