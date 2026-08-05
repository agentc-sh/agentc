// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[derive(Clone)]
pub struct DefaultTenantId(String);

impl DefaultTenantId {
    pub fn new(default_tenant_id: impl Into<String>) -> Self {
        Self(default_tenant_id.into())
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}
