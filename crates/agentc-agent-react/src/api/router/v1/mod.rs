// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod messages;
pub mod runs;
pub mod sessions;

use jobq::{AnyExecutable, FifoQueue, JobQueue};
use std::sync::Arc;
use subway::Bus;
use utoipa_axum::router::OpenApiRouter;

use agentc_http::state::DefaultTenantId;

use crate::{api::state::ReActApiState, service::ApplicationService};

pub fn router(
    service: Arc<ApplicationService>,
    default_tenant_id: DefaultTenantId,
    task_queue: Arc<JobQueue<FifoQueue<AnyExecutable>>>,
    bus: Bus,
) -> OpenApiRouter {
    OpenApiRouter::new().nest("/v1", {
        OpenApiRouter::new()
            .merge(sessions::router())
            .merge(messages::router())
            .merge(runs::router())
            .with_state(ReActApiState::new(service, default_tenant_id, task_queue, bus))
    })
}
