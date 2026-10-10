// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde_json::Value;

use agentc_blocks::context::ResolvedContextProviderKind;

use crate::manifest::{ManifestProviderParams, errors::ManifestError};

pub trait ManifestProviderModel {
    fn name(&self) -> &str;

    fn params(&self) -> Option<&ManifestProviderParams>;
}

pub trait ResolveProviderKind {
    fn resolve_kind(&self, locals: &Value) -> Result<ResolvedContextProviderKind, ManifestError>;
}
