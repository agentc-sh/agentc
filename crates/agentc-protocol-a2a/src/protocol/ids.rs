// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{
    Deserialize,
    Serialize,
};
use std::{
    fmt::{
        Display,
        Formatter,
        Result as FmtResult,
    },
    ops::Deref,
};
use utoipa::ToSchema;
use uuid::{
    Error as UuidError,
    Uuid,
};

macro_rules! define_id_type {
    ($name:ident) => {
        #[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
        #[serde(default, transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
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
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
                formatter.write_str(&self.0)
            }
        }
    };
}

define_id_type!(ArtifactId);
define_id_type!(TaskId);

impl TryFrom<TaskId> for Uuid {
    type Error = UuidError;

    fn try_from(value: TaskId) -> Result<Self, Self::Error> {
        Uuid::parse_str(value.as_ref())
    }
}

impl TryFrom<&TaskId> for Uuid {
    type Error = UuidError;

    fn try_from(value: &TaskId) -> Result<Self, Self::Error> {
        Uuid::parse_str(value.as_ref())
    }
}
