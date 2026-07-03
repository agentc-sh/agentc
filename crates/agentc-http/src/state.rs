// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{ops::Deref, sync::Arc};

#[derive(Clone)]
pub struct ApiState<S> {
    pub service: Arc<S>,
    pub default_tenant_id: String,
}

impl<S> ApiState<S> {
    pub fn new(service: S, default_tenant_id: impl Into<String>) -> Self {
        Self {
            service: Arc::new(service),
            default_tenant_id: default_tenant_id.into(),
        }
    }

    pub fn new_arc(service: Arc<S>, default_tenant_id: impl Into<String>) -> Self {
        Self {
            service,
            default_tenant_id: default_tenant_id.into(),
        }
    }
}

impl<S> Deref for ApiState<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.service
    }
}
