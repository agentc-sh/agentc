// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Context {
    /// A description of the context item
    pub description: String,
    /// The value of the context item
    pub value: String,
}

impl Context {
    pub fn new(description: String, value: String) -> Self {
        Self { description, value }
    }
}
