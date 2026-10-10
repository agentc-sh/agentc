// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::types::identity::ProviderKind;

pub const KIND: ProviderKind = ProviderKind::new("openrouter");

pub const OTEL_PROVIDER_NAME: &str = KIND.as_str();

pub const API_KEY_ENV: &str = "OPENROUTER_API_KEY";
