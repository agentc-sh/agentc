// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::types::model::ModelConfig;

#[derive(Debug, Clone, Default)]
pub struct ReActGraphConfig {
    pub default_model_config: ModelConfig,
}
