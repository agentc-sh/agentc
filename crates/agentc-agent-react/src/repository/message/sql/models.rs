// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod message {
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use serde_json::{Value, from_value, json};
    use std::str::FromStr;
    use uuid::Uuid;

    use agentc_database::{
        errors::DatabaseError,
        json::Json,
        orm::{ActiveValue, prelude::*},
        paginate::{CursorValue, ExtractCursorValue},
    };

    use crate::types::message::{
        AssistantMessage, Message, MessageRole, ReasoningMessage, SystemMessage, ToolMessage,
        UserContent, UserMessage,
    };

    #[derive(Debug, Clone, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "message")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: String,
        pub session_id: Uuid,
        pub run_id: Option<Uuid>,
        pub checkpoint_id: Option<Uuid>,
        pub role: String,
        pub content: Option<String>,
        pub data: Option<Json<Value>>,
        pub created_at: DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    #[async_trait]
    impl ActiveModelBehavior for ActiveModel {}

    impl ExtractCursorValue for Model {
        fn extract_cursor_value(&self, field_name: &str) -> Result<CursorValue, DatabaseError> {
            match field_name {
                "id" => Ok(CursorValue::Uuid(Some(self.id))),
                "tenant_id" => Ok(CursorValue::String(Some(self.tenant_id.clone()))),
                "session_id" => Ok(CursorValue::Uuid(Some(self.session_id))),
                "run_id" => Ok(CursorValue::Uuid(self.run_id)),
                "checkpoint_id" => Ok(CursorValue::Uuid(self.checkpoint_id)),
                "role" => Ok(CursorValue::String(Some(self.role.clone()))),
                "created_at" => Ok(CursorValue::DateTime(Some(self.created_at))),
                _ => Err(DatabaseError::UnknownFieldName(field_name.to_string())),
            }
        }
    }

    impl TryFrom<Model> for Message {
        type Error = String;

        fn try_from(model: Model) -> Result<Self, Self::Error> {
            match MessageRole::from_str(model.role.as_str())
                .map_err(|_| format!("Invalid message role: {}", model.role))?
            {
                MessageRole::System => Ok(Message::System(SystemMessage {
                    id: model.id,
                    tenant_id: model.tenant_id,
                    session_id: model.session_id,
                    run_id: model.run_id,
                    checkpoint_id: model.checkpoint_id,
                    content: model.content.unwrap_or_default(),
                    name: None,
                    created_at: model.created_at,
                })),
                MessageRole::User => Ok(Message::User(UserMessage {
                    id: model.id,
                    tenant_id: model.tenant_id,
                    session_id: model.session_id,
                    run_id: model.run_id,
                    checkpoint_id: model.checkpoint_id,
                    content: model
                        .data
                        .ok_or_else(|| "Missing data for user message".to_string())?
                        .get("content")
                        .cloned()
                        .ok_or_else(|| "Missing content in user message data".to_string())
                        .and_then(|value| {
                            from_value::<Vec<UserContent>>(value).map_err(|error| error.to_string())
                        })?,
                    name: None,
                    created_at: model.created_at,
                })),
                MessageRole::Assistant => Ok(Message::Assistant(AssistantMessage {
                    id: model.id,
                    tenant_id: model.tenant_id,
                    session_id: model.session_id,
                    run_id: model
                        .run_id
                        .ok_or_else(|| "Missing run_id for assistant message".to_string())?,
                    checkpoint_id: model.checkpoint_id,
                    content: model.content,
                    name: None,
                    tool_calls: model
                        .data
                        .and_then(|data| data.get("tool_calls").cloned())
                        .and_then(|value| {
                            from_value(value)
                                .map_err(|e| e.to_string())
                                .ok()
                        }),
                    created_at: model.created_at,
                })),
                MessageRole::Tool => Ok(Message::Tool(ToolMessage {
                    id: model.id,
                    tenant_id: model.tenant_id,
                    session_id: model.session_id,
                    run_id: model.run_id,
                    checkpoint_id: model.checkpoint_id,
                    content: model.content,
                    name: None,
                    tool_call_id: model
                        .data
                        .clone()
                        .and_then(|data| data.get("tool_call_id").cloned())
                        .and_then(|value| value.as_str().map(|s| s.to_string()))
                        .ok_or("Missing or invalid tool_call_id in data")?,
                    parent_message_id: model
                        .data
                        .clone()
                        .and_then(|data| data.get("parent_message_id").cloned())
                        .and_then(|value| value.as_str().map(String::from))
                        .and_then(|s| Uuid::from_str(&s).ok()),
                    error: model
                        .data
                        .and_then(|data| data.get("error").cloned())
                        .and_then(|value| value.as_str().map(String::from)),
                    created_at: model.created_at,
                })),
                MessageRole::Reasoning => Ok(Message::Reasoning(ReasoningMessage {
                    id: model.id,
                    tenant_id: model.tenant_id,
                    session_id: model.session_id,
                    run_id: model
                        .run_id
                        .ok_or_else(|| "Missing run_id for reasoning message".to_string())?,
                    checkpoint_id: model.checkpoint_id,
                    content: model.content.unwrap_or_default(),
                    signature: model
                        .data
                        .and_then(|data| data.get("signature").cloned())
                        .and_then(|value| value.as_str().map(String::from)),
                    created_at: model.created_at,
                })),
            }
        }
    }

    impl TryFrom<Message> for ActiveModel {
        type Error = String;

        fn try_from(message: Message) -> Result<Self, Self::Error> {
            Ok(ActiveModel {
                id: ActiveValue::set(*message.id()),
                tenant_id: ActiveValue::set(message.tenant_id().to_string()),
                session_id: ActiveValue::set(*message.session_id()),
                run_id: if let Some(run_id) = message.run_id() {
                    ActiveValue::set(Some(*run_id))
                } else {
                    ActiveValue::not_set()
                },
                checkpoint_id: if let Some(checkpoint_id) = message.checkpoint_id() {
                    ActiveValue::set(Some(*checkpoint_id))
                } else {
                    ActiveValue::not_set()
                },
                role: ActiveValue::set(message.role().to_string()),
                content: ActiveValue::set(message.content().map(str::to_string)),
                data: match &message {
                    Message::System(_) => ActiveValue::set(None),
                    Message::User(message) => ActiveValue::set(Some(Json(json!({
                        "content": message.content,
                    })))),
                    Message::Assistant(message) => {
                        ActiveValue::set(Some(Json(json!({ "tool_calls": message.tool_calls }))))
                    }
                    Message::Tool(message) => ActiveValue::set(Some(Json(json!({
                        "tool_call_id": message.tool_call_id,
                        "error": message.error,
                        "parent_message_id": message.parent_message_id,
                    })))),
                    Message::Reasoning(message) => ActiveValue::set(Some(Json(json!({
                        "signature": message.signature,
                    })))),
                },
                created_at: ActiveValue::set(message.created_at().to_owned()),
            })
        }
    }
}
