// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::guestjs::{FromGuest, ToGuest};
use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, FromGuest, ToGuest)]
#[guestjs(crate_path = agentc_executor_typescript::guestjs)]
pub struct ToolOutput {
    pub output: Value,

    #[serde(default)]
    pub state_update: Option<json_patch::Patch>,
}
