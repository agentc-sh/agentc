// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

mod errors;
mod config;
mod base;
mod constants;
mod call;

use reqwest::{
    header::{
        ACCEPT,
        CONTENT_TYPE,
    },
    Response,
};
use reqwest_sse::EventSource;
use futures::{
    stream::LocalBoxStream,
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
        errors::A2aClientError,
        config::A2aClientConfig,
        base::BaseClient,
        call::Call,
        constants::{
            A2A_CONTENT_TYPE,
            A2A_TENANT_HEADER,
            A2A_VERSION,
            A2A_VERSION_HEADER,
        },
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
    ) -> Call<'_, Response, LocalBoxStream<'static, Result<StreamResponse, A2aClientError>>> {
        Call::post(&self.client, "/message:stream")
            .header_lossy(A2A_VERSION_HEADER, A2A_VERSION)
            .header_lossy(CONTENT_TYPE, A2A_CONTENT_TYPE)
            .header_lossy(ACCEPT, "text/event-stream")
            .maybe_header_lossy(A2A_TENANT_HEADER, request.tenant.as_deref())
            .body(&request)
            .map(|response| async move {
                response
                    .events()
                    .await
                    .map_err(|err| A2aClientError::stream_decode(err.to_string()))
                    .map(|events| {
                        events
                            .map_err(|err| A2aClientError::stream_decode(err.to_string()))
                            .and_then(|event| async move {
                                if event.event_type == "error" {
                                    return Err(A2aClientError::stream_decode(event.data));
                                }

                                serde_json::from_str::<StreamResponse>(&event.data)
                                    .map_err(|err| {
                                        A2aClientError::stream_decode(err.to_string())
                                    })
                            })
                            .boxed_local()
                    })
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
