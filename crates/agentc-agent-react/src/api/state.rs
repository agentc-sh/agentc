// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;
use jobq::{
    AnyExecutable,
    FifoQueue,
    JobQueue,
};

use agentc_http::state::DefaultTenantId;

use crate::service::ApplicationService;

#[derive(Clone)]
pub struct ReActApiState {
    pub service: Arc<ApplicationService>,
    pub default_tenant_id: DefaultTenantId,
    pub task_queue: Arc<JobQueue<FifoQueue<AnyExecutable>>>,
}

impl ReActApiState {
    pub fn new(
        service: Arc<ApplicationService>,
        default_tenant_id: DefaultTenantId,
        task_queue: Arc<JobQueue<FifoQueue<AnyExecutable>>>,
    ) -> Self {
        Self {
            service,
            default_tenant_id,
            task_queue,
        }
    }
}
