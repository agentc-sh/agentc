// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::ops::{Deref, DerefMut};

use guestjs::{FromGuest, ToGuest};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Arbitrary JSON marshalled to and from guest values.
#[derive(Debug, Clone, Serialize, Deserialize, ToGuest, FromGuest)]
#[serde(transparent)]
pub struct Json(pub Value);

impl Json {
    pub fn as_inner(&self) -> &Value {
        &self.0
    }

    pub fn into_inner(self) -> Value {
        self.0
    }
}

impl Deref for Json {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Json {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
