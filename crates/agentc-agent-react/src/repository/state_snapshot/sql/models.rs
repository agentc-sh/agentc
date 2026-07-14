// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod state_snapshot {
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use serde_json::Value;
    use uuid::Uuid;

    use agentc_agent::types::{capability::CapabilityOverride, tools::ToolDefinition};
    use agentc_database::{
        errors::DatabaseError,
        json::Json,
        orm::{ActiveValue, prelude::*},
        paginate::{CursorValue, ExtractCursorValue},
    };

    use crate::types::{
        context_var::ContextVar, model::ModelConfig, state_snapshot::StateSnapshot,
    };

    #[derive(Debug, Clone, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "state_snapshot")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: String,
        pub session_id: Uuid,
        pub run_id: Uuid,
        pub checkpoint_id: Option<Uuid>,
        pub model: Option<Json<ModelConfig>>,
        pub capability_override: Option<Json<CapabilityOverride>>,
        pub tools: Option<Json<Vec<ToolDefinition>>>,
        pub context_vars: Option<Json<Vec<ContextVar>>>,
        pub context: Option<Json<Value>>,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
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
                "run_id" => Ok(CursorValue::Uuid(Some(self.run_id))),
                "checkpoint_id" => Ok(CursorValue::Uuid(self.checkpoint_id)),
                "created_at" => Ok(CursorValue::DateTime(Some(self.created_at))),
                "updated_at" => Ok(CursorValue::DateTime(Some(self.updated_at))),
                _ => Err(DatabaseError::UnknownFieldName(field_name.to_string())),
            }
        }
    }

    impl TryFrom<Model> for StateSnapshot {
        type Error = String;

        fn try_from(model: Model) -> Result<Self, Self::Error> {
            Ok(StateSnapshot {
                id: model.id,
                tenant_id: model.tenant_id,
                session_id: model.session_id,
                run_id: model.run_id,
                checkpoint_id: model.checkpoint_id,
                model: model
                    .model
                    .map(|json| json.into_inner()),
                capability_override: model
                    .capability_override
                    .map(|json| json.into_inner()),
                tools: model
                    .tools
                    .map(|json| json.into_inner()),
                context_vars: model
                    .context_vars
                    .map(|json| json.into_inner()),
                context: model
                    .context
                    .map(|json| json.into_inner())
                    .unwrap_or(Value::Object(Default::default())),
                created_at: model.created_at,
                updated_at: model.updated_at,
            })
        }
    }

    impl TryFrom<StateSnapshot> for ActiveModel {
        type Error = String;

        fn try_from(snapshot: StateSnapshot) -> Result<Self, Self::Error> {
            Ok(ActiveModel {
                id: ActiveValue::set(snapshot.id),
                tenant_id: ActiveValue::set(snapshot.tenant_id),
                session_id: ActiveValue::set(snapshot.session_id),
                run_id: ActiveValue::set(snapshot.run_id),
                checkpoint_id: ActiveValue::set(snapshot.checkpoint_id),
                model: ActiveValue::set(snapshot.model.map(Json)),
                capability_override: ActiveValue::set(snapshot.capability_override.map(Json)),
                tools: ActiveValue::set(snapshot.tools.map(Json)),
                context_vars: ActiveValue::set(snapshot.context_vars.map(Json)),
                context: ActiveValue::set(Some(Json(snapshot.context))),
                created_at: ActiveValue::set(snapshot.created_at),
                updated_at: ActiveValue::set(snapshot.updated_at),
            })
        }
    }
}
