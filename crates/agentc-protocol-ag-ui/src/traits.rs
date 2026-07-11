// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use futures::{Stream, stream::BoxStream};
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use agentc_http::errors::ApiError;

use crate::protocol::{event::Event, input::RunAgentInput};

#[async_trait]
pub trait AgUiRunCancel: Send + Sync {
    async fn cancel(&self) -> Result<(), ApiError>;
}

pub struct AgUiRunStream {
    inner: BoxStream<'static, Result<Event, ApiError>>,
    cancel: Option<Arc<dyn AgUiRunCancel>>,
}

impl AgUiRunStream {
    pub fn new(inner: BoxStream<'static, Result<Event, ApiError>>) -> Self {
        Self { inner, cancel: None }
    }

    pub fn with_cancel<C>(mut self, cancel: C) -> Self
    where
        C: AgUiRunCancel + 'static,
    {
        self.cancel = Some(Arc::new(cancel));
        self
    }

    pub async fn cancel(&mut self) -> Result<(), ApiError> {
        if let Some(cancel) = self.cancel.clone() {
            cancel.cancel().await?;
        }

        Ok(())
    }
}

impl Stream for AgUiRunStream {
    type Item = Result<Event, ApiError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

/// Trait that any agent service must implement to serve the AG-UI protocol
/// in the HTTP server.
#[async_trait]
pub trait AgUiService: Send + Sync {
    async fn ag_ui_run(
        &self,
        input: RunAgentInput,
        tenant_id: &str,
    ) -> Result<AgUiRunStream, ApiError>;
}

pub trait FromAgUiType<T>: Sized {
    type Error: std::error::Error + Send + Sync + 'static;

    fn from_ag_ui_type(value: T) -> Result<Self, Self::Error>;
}

pub trait ToAgUiType<T> {
    type Error: std::error::Error + Send + Sync + 'static;

    fn to_ag_ui_type(self) -> Result<T, Self::Error>;
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use futures::{StreamExt, stream};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use agentc_http::errors::ApiError;

    use crate::protocol::event::Event;

    use super::{AgUiRunCancel, AgUiRunStream};

    struct TestCancel {
        called: Arc<AtomicBool>,
    }

    #[async_trait]
    impl AgUiRunCancel for TestCancel {
        async fn cancel(&self) -> Result<(), ApiError> {
            self.called
                .store(true, Ordering::SeqCst);

            Ok(())
        }
    }

    #[tokio::test]
    async fn ag_ui_run_stream_cancel_invokes_hook() {
        let called = Arc::new(AtomicBool::new(false));
        let mut stream = AgUiRunStream::new(stream::empty::<Result<Event, ApiError>>().boxed())
            .with_cancel(TestCancel { called: called.clone() });

        stream.cancel().await.unwrap();

        assert!(called.load(Ordering::SeqCst));
    }
}
