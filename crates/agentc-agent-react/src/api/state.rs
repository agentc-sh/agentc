// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use jobq::{AnyExecutable, BatchPolicy, FifoQueue, JobQueue};
use std::sync::Arc;
use subway::Bus;

use agentc_http::server::state::DefaultTenantId;

use crate::service::ApplicationService;

#[derive(Clone)]
pub struct ReActApiState {
    pub service: Arc<ApplicationService>,
    pub default_tenant_id: DefaultTenantId,
    pub task_queue: JobQueue<FifoQueue<AnyExecutable>>,
    pub batch_policy: BatchPolicy,
    pub bus: Bus,
}

impl ReActApiState {
    pub fn new(
        service: Arc<ApplicationService>,
        default_tenant_id: DefaultTenantId,
        task_queue: JobQueue<FifoQueue<AnyExecutable>>,
        batch_policy: BatchPolicy,
        bus: Bus,
    ) -> Self {
        Self {
            service,
            default_tenant_id,
            task_queue,
            batch_policy,
            bus,
        }
    }
}
