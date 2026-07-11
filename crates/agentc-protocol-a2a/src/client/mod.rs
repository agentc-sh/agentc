// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

mod base;
mod call;
mod config;
mod constants;
mod errors;
mod sse;

pub use config::A2aClientConfig;
pub use constants::{A2A_CONTENT_TYPE, A2A_TENANT_HEADER, A2A_VERSION, A2A_VERSION_HEADER};
pub use errors::A2aClientError;

use futures::{StreamExt, TryStreamExt, stream::BoxStream};
use reqwest::{
    Response,
    header::{ACCEPT, CONTENT_TYPE},
};

use crate::{
    client::{
        base::BaseClient,
        call::Call,
        sse::{Item, Sse},
    },
    protocol::{
        AgentCard, CancelTaskRequest, GetTaskRequest, SendMessageRequest, SendMessageResponse,
        StreamResponse, Task,
    },
};

#[derive(Debug, Clone)]
pub struct A2aClient {
    client: BaseClient,
}

impl A2aClient {
    pub fn new(config: A2aClientConfig) -> Result<Self, A2aClientError> {
        Ok(Self { client: BaseClient::from_config(config)? })
    }

    pub fn agent_card(&self) -> Call<'_, Response, AgentCard> {
        Call::get(&self.client, "/.well-known/agent-card.json")
            .header_lossy(A2A_VERSION_HEADER, A2A_VERSION)
            .header_lossy(ACCEPT, A2A_CONTENT_TYPE)
            .timeout(self.client.config().timeout)
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
            .timeout(self.client.config().timeout)
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
                Ok(response
                    .sse()
                    .try_filter_map(|item| async move {
                        match item {
                            Item::Comment(_) => Ok(None),
                            Item::Event(event) if event.event_type.as_deref() == Some("error") => {
                                Err(A2aClientError::stream_decode(event.data))
                            }
                            Item::Event(event) => {
                                serde_json::from_str::<StreamResponse>(&event.data)
                                    .map(Some)
                                    .map_err(|e| A2aClientError::stream_decode(e.to_string()))
                            }
                        }
                    })
                    .boxed())
            })
    }

    pub fn get_task(&self, request: GetTaskRequest) -> Call<'_, Response, Task> {
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
            .timeout(self.client.config().timeout)
            .params(&GetTaskParams { history_length: request.history_length })
            .json()
    }

    pub fn cancel_task(&self, request: CancelTaskRequest) -> Call<'_, Response, Task> {
        Call::post(&self.client, format!("/tasks/{}:cancel", request.id.as_ref()))
            .header_lossy(A2A_VERSION_HEADER, A2A_VERSION)
            .header_lossy(CONTENT_TYPE, A2A_CONTENT_TYPE)
            .header_lossy(ACCEPT, A2A_CONTENT_TYPE)
            .maybe_header_lossy(A2A_TENANT_HEADER, request.tenant.as_deref())
            .timeout(self.client.config().timeout)
            .body(&request)
            .json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures::TryStreamExt;
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_partial_json, header, method, path},
    };

    use crate::protocol::{
        Message, Part, Role, SendMessageRequest, SendMessageResponse, StreamResponse, Task, TaskId,
        TaskState, TaskStatus,
    };

    struct ClientFixture;

    impl ClientFixture {
        async fn client() -> (MockServer, A2aClient) {
            let server = MockServer::start().await;
            let client = A2aClient::new(A2aClientConfig::new(server.uri()))
                .expect("client config should be valid");

            (server, client)
        }

        fn request() -> SendMessageRequest {
            SendMessageRequest {
                message: Message {
                    context_id: Some("context-1".to_string()),
                    ..Message::new(Role::User, vec![Part::text("plan this")])
                },
                configuration: None,
                metadata: None,
                tenant: Some("tenant-1".to_string()),
            }
        }

        fn task(state: TaskState) -> Task {
            Task {
                id: TaskId::new("task-1"),
                context_id: "context-1".to_string(),
                status: TaskStatus { state, message: None, timestamp: None },
                artifacts: None,
                history: None,
                metadata: None,
            }
        }
    }

    #[tokio::test]
    async fn send_message_sets_protocol_headers_tenant_and_decodes_response() {
        let (server, client) = ClientFixture::client().await;

        Mock::given(method("POST"))
            .and(path("/message:send"))
            .and(header(A2A_VERSION_HEADER, A2A_VERSION))
            .and(header(A2A_TENANT_HEADER, "tenant-1"))
            .and(header(CONTENT_TYPE.as_str(), A2A_CONTENT_TYPE))
            .and(header(ACCEPT.as_str(), A2A_CONTENT_TYPE))
            .and(body_partial_json(json!({
                "tenant": "tenant-1",
                "message": {
                    "contextId": "context-1",
                    "role": "ROLE_USER",
                },
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(SendMessageResponse::Task(
                ClientFixture::task(TaskState::Submitted),
            )))
            .mount(&server)
            .await;

        assert!(matches!(
            client
                .send_message(ClientFixture::request())
                .await
                .expect("send should succeed"),
            SendMessageResponse::Task(task) if task.id.as_ref() == "task-1"
        ));
    }

    #[tokio::test]
    async fn stream_message_ignores_comments_and_decodes_structured_events() {
        let (server, client) = ClientFixture::client().await;

        Mock::given(method("POST"))
            .and(path("/message:stream"))
            .and(header(A2A_VERSION_HEADER, A2A_VERSION))
            .and(header(A2A_TENANT_HEADER, "tenant-1"))
            .and(header(ACCEPT.as_str(), "text/event-stream"))
            .respond_with(
                ResponseTemplate::new(200)
                    .append_header(CONTENT_TYPE.as_str(), "text/event-stream")
                    .set_body_string(format!(
                        ": keep-alive\n\ndata: {}\n\n",
                        serde_json::to_string(&StreamResponse::Task(ClientFixture::task(
                            TaskState::Completed
                        ),))
                        .expect("stream response should serialize"),
                    )),
            )
            .mount(&server)
            .await;

        assert!(matches!(
            client
                .stream_message(ClientFixture::request())
                .await
                .expect("stream should open")
                .try_next()
                .await
                .expect("stream event should decode"),
            Some(StreamResponse::Task(task)) if task.status.state == TaskState::Completed
        ));
    }
}
