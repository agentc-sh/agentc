// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use agentc_executor_typescript::{
    backend::ExecutorBackend,
    guestjs::{FromGuest, enter::Enter, errors::Error, marshal::args::Args},
};
use bytes::Bytes;
use serde::{Deserialize, Serialize};

/// The second argument to the guest `fetch` function.
#[derive(Debug, Default, Deserialize, FromGuest)]
#[guestjs(crate_path = agentc_executor_typescript::guestjs)]
#[serde(rename_all = "camelCase")]
pub struct FetchInit {
    pub method: Option<String>,
    pub headers: Option<FetchHeaders>,
    pub body: Option<FetchBody>,
}

/// A header record supplied by guest code.
#[derive(Debug, Default, Deserialize, FromGuest)]
#[guestjs(crate_path = agentc_executor_typescript::guestjs)]
#[serde(transparent)]
pub struct FetchHeaders(pub BTreeMap<String, String>);

/// A request body supplied by guest code.
#[derive(Debug, Deserialize, Serialize, FromGuest)]
#[guestjs(union, crate_path = agentc_executor_typescript::guestjs)]
#[serde(untagged)]
pub enum FetchBody {
    Bytes(Bytes),
    Text(String),
}

impl From<FetchBody> for Bytes {
    fn from(body: FetchBody) -> Self {
        match body {
            FetchBody::Text(text) => Bytes::from(text.into_bytes()),
            FetchBody::Bytes(bytes) => bytes,
        }
    }
}

pub(crate) struct FetchRequest {
    pub(crate) url: String,
    pub(crate) init: FetchInit,
}

impl FetchRequest {
    pub(crate) fn read<'js, B: ExecutorBackend>(
        enter: &Enter<'js, B>,
        args: &Args<'js, B>,
    ) -> Result<Self, Error> {
        Ok(
            Self {
                url: args.next::<String>(enter)?,
                init: args
                    .next::<Option<FetchInit>>(enter)?
                    .unwrap_or_default(),
            }
        )
    }
}
