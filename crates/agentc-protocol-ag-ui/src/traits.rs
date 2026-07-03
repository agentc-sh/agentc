// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::protocol::{event::Event, input::RunAgentInput};

/// Trait that any agent service must implement to serve the AG-UI protocol
/// in the HTTP server.
#[async_trait]
pub trait AgUiService: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn ag_ui_run(
        &self,
        input: RunAgentInput,
        tenant_id: &str,
    ) -> Result<BoxStream<'static, Result<Event, Self::Error>>, Self::Error>;
}

pub trait FromAgUiType<T>: Sized {
    type Error: std::error::Error + Send + Sync + 'static;

    fn from_ag_ui_type(value: T) -> Result<Self, Self::Error>;
}

pub trait ToAgUiType<T> {
    type Error: std::error::Error + Send + Sync + 'static;

    fn to_ag_ui_type(self) -> Result<T, Self::Error>;
}
