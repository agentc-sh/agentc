// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::ops::Deref;
use utoipa::ToSchema;
use uuid::Uuid;

/// Macro to define a newtype ID based on a string.
macro_rules! define_id_type {
    // This arm of the macro handles calls that don't specify extra derives.
    ($name:ident) => {
        define_id_type!($name,);
    };
    // This arm handles calls that do specify extra derives (like Eq).
    ($name:ident, $($extra_derive:ident),*) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, $($extra_derive),*)]
        pub struct $name(String);

        impl $name {
            /// Creates a new random ID.
            pub fn random() -> Self {
                Self(Uuid::new_v4().to_string())
            }
        }

        /// Allows creating an ID from a Uuid.
        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid.to_string())
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_string())
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        /// Allows printing the ID.
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

define_id_type!(AgentId, ToSchema);
define_id_type!(ThreadId, ToSchema);
define_id_type!(RunId, ToSchema);
define_id_type!(MessageId, ToSchema);
define_id_type!(InterruptId, ToSchema);

/// A tool call ID.
/// Used by some providers to denote a specific ID for a tool call generation, where the result of the tool call must also use this ID.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone, ToSchema)]
pub struct ToolCallId(String);

/// Tool Call ID
///
/// Does not follow UUID format, instead uses "call_xxxxxxxx"
impl ToolCallId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn random() -> Self {
        let uuid = &Uuid::new_v4().to_string()[..8];
        let id = format!("call_{uuid}");
        Self(id)
    }
}

impl Deref for ToolCallId {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for ToolCallId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ToolCallId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for ToolCallId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<ToolCallId> for String {
    fn from(value: ToolCallId) -> Self {
        value.0
    }
}
