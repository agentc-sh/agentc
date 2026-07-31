// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::Serialize;

/// Converts a concrete runtime value into its OpenTelemetry GenAI form.
pub trait ToGenAiType {
    type GenAiType: Serialize;

    fn to_gen_ai_type(&self) -> Result<Self::GenAiType, serde_json::Error>;
}
