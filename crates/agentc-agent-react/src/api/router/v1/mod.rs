// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod messages;
pub mod runs;
pub mod sessions;

use utoipa_axum::router::OpenApiRouter;

use agentc_http::state::ApiState;

use crate::service::ApplicationService;

pub fn router(state: ApiState<ApplicationService>) -> OpenApiRouter {
    OpenApiRouter::new().nest("/v1", {
        OpenApiRouter::new()
            .merge(sessions::router())
            .merge(messages::router())
            .merge(runs::router())
            .with_state(state)
    })
}
