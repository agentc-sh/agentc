// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use futures::{
    Stream,
    stream::BoxStream,
};
use std::{
    pin::Pin,
    sync::Arc,
    task::{
        Context,
        Poll,
    },
};

use agentc_http::errors::ApiError;

use crate::protocol::{
    AgentCard,
    AgentInterface,
    CancelTaskRequest,
    GetTaskRequest,
    SendMessageRequest,
    SendMessageResponse,
    StreamResponse,
    Task,
};

#[async_trait]
pub trait A2aRunCancel: Send + Sync {
    async fn cancel(&self) -> Result<(), ApiError>;
}

pub struct A2aStream {
    inner: BoxStream<'static, Result<StreamResponse, ApiError>>,
    cancel: Option<Arc<dyn A2aRunCancel>>,
}

impl A2aStream {
    pub fn new(
        inner: BoxStream<'static, Result<StreamResponse, ApiError>>,
    ) -> Self {
        Self {
            inner,
            cancel: None,
        }
    }

    pub fn with_cancel<C>(mut self, cancel: C) -> Self
    where
        C: A2aRunCancel + 'static,
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

impl Stream for A2aStream {
    type Item = Result<StreamResponse, ApiError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

/// Service contract for exposing an agent through the server-side A2A protocol.
#[async_trait]
pub trait A2aService: Send + Sync {
    fn agent_card(&self, interface: &AgentInterface) -> AgentCard;

    async fn send_message(
        &self,
        request: SendMessageRequest,
    ) -> Result<SendMessageResponse, ApiError>;

    async fn stream_message(
        &self,
        request: SendMessageRequest,
    ) -> Result<A2aStream, ApiError>;

    async fn get_task(
        &self,
        request: GetTaskRequest,
    ) -> Result<Task, ApiError>;

    async fn cancel_task(
        &self,
        request: CancelTaskRequest,
    ) -> Result<Task, ApiError>;
}

pub trait FromA2aType<T>: Sized {
    type Error: std::error::Error + Send + Sync + 'static;

    fn from_a2a_type(value: T) -> Result<Self, Self::Error>;
}

pub trait ToA2aType<T> {
    type Error: std::error::Error + Send + Sync + 'static;

    fn to_a2a_type(self) -> Result<T, Self::Error>;
}
