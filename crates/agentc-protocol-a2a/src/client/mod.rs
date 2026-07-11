// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

mod errors;
mod config;
mod base;
mod constants;
mod call;
mod sse;

pub use config::A2aClientConfig;
pub use constants::{
    A2A_CONTENT_TYPE,
    A2A_TENANT_HEADER,
    A2A_VERSION,
    A2A_VERSION_HEADER,
};
pub use errors::A2aClientError;

use reqwest::{
    header::{
        ACCEPT,
        CONTENT_TYPE,
    },
    Response,
};
use futures::{
    stream::BoxStream,
    StreamExt,
    TryStreamExt,
};

use crate::{
    protocol::{
        AgentCard,
        CancelTaskRequest,
        GetTaskRequest,
        SendMessageRequest,
        SendMessageResponse,
        StreamResponse,
        Task,
    },
    client::{
        base::BaseClient,
        call::Call,
        sse::{Sse, Item},
    },
};


#[derive(Debug, Clone)]
pub struct A2aClient {
    client: BaseClient,
}

impl A2aClient {
    pub fn new(config: A2aClientConfig) -> Result<Self, A2aClientError> {
        Ok(Self {
            client: BaseClient::from_config(config)?,
        })
    }

    pub fn agent_card(&self) -> Call<'_, Response, AgentCard> {
        Call::get(&self.client, "/.well-known/agent-card.json")
            .header_lossy(A2A_VERSION_HEADER, A2A_VERSION)
            .header_lossy(ACCEPT, A2A_CONTENT_TYPE)
            .json()
    }

    pub fn send_message(
        &self,
        request: SendMessageRequest,
    ) -> Call<'_, Response, SendMessageResponse> {
        Call::post(&self.client, "/message:send")
            .header_lossy(A2A_VERSION_HEADER, A2A_VERSION)
            .header_lossy(CONTENT_TYPE, A2A_CONTENT_TYPE)
            .header_lossy(ACCEPT, A2A_CONTENT_TYPE)
            .maybe_header_lossy(A2A_TENANT_HEADER, request.tenant.as_deref())
            .body(&request)
            .json()
    }

    pub fn stream_message(
        &self,
        request: SendMessageRequest,
    ) -> Call<'_, Response, BoxStream<'static, Result<StreamResponse, A2aClientError>>> {
        Call::post(&self.client, "/message:stream")
            .header_lossy(A2A_VERSION_HEADER, A2A_VERSION)
            .header_lossy(CONTENT_TYPE, A2A_CONTENT_TYPE)
            .header_lossy(ACCEPT, "text/event-stream")
            .maybe_header_lossy(A2A_TENANT_HEADER, request.tenant.as_deref())
            .body(&request)
            .map(|response| async move {
                Ok(
                    response
                        .sse()
                        .map_err(|e| A2aClientError::stream_decode(e.to_string()))
                        .try_filter_map(|item| async move {
                            match item {
                                Item::Comment(_) => Ok(None),
                                Item::Event(event) if event.event_type.as_deref() == Some("error") => Err(
                                    A2aClientError::stream_decode(event.data),
                                ),
                                Item::Event(event) => serde_json::from_str::<StreamResponse>(&event.data)
                                    .map(Some)
                                    .map_err(|e| A2aClientError::stream_decode(e.to_string())),
                            }
                        })
                        .boxed()
                )
            })
    }

    pub fn get_task(
        &self,
        request: GetTaskRequest,
    ) -> Call<'_, Response, Task> {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct GetTaskParams {
            #[serde(skip_serializing_if = "Option::is_none")]
            history_length: Option<i32>,
        }

        Call::get(&self.client, format!("/tasks/{}", request.id.as_ref()))
            .header_lossy(A2A_VERSION_HEADER, A2A_VERSION)
            .header_lossy(ACCEPT, A2A_CONTENT_TYPE)
            .maybe_header_lossy(A2A_TENANT_HEADER, request.tenant.as_deref())
            .params(&GetTaskParams {
                history_length: request.history_length,
            })
            .json()
    }

    pub fn cancel_task(
        &self,
        request: CancelTaskRequest,
    ) -> Call<'_, Response, Task> {
        Call::post(&self.client, format!("/tasks/{}:cancel", request.id.as_ref()))
            .header_lossy(A2A_VERSION_HEADER, A2A_VERSION)
            .header_lossy(CONTENT_TYPE, A2A_CONTENT_TYPE)
            .header_lossy(ACCEPT, A2A_CONTENT_TYPE)
            .maybe_header_lossy(A2A_TENANT_HEADER, request.tenant.as_deref())
            .body(&request)
            .json()
    }
}
