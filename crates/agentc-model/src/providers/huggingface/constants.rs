// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::types::identity::StaticProviderId;

pub const PROVIDER: StaticProviderId = StaticProviderId::new("huggingface");

pub const OTEL_PROVIDER_NAME: &str = PROVIDER.as_str();
