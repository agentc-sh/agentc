// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use agentc_executor_typescript::guestjs::{
    FromGuest,
    errors::Error,
    handle::{BoundObject, Object},
    host::Args,
    runtime::Scope,
};
use bytes::Bytes;
use serde::Deserialize;

/// The second argument to the guest `fetch` function.
#[derive(Debug, Default, Deserialize, FromGuest)]
#[guestjs(crate_path = agentc_executor_typescript::guestjs)]
#[serde(rename_all = "camelCase")]
pub struct FetchInit {
    pub method: Option<String>,
    pub headers: Option<FetchHeaders>,
}

/// A header record supplied by guest code.
#[derive(Debug, Default, Deserialize, FromGuest)]
#[guestjs(crate_path = agentc_executor_typescript::guestjs)]
#[serde(transparent)]
pub struct FetchHeaders(pub BTreeMap<String, String>);

/// A request body supplied by guest code.
pub enum FetchBody {
    Text(String),
    Bytes(Bytes),
}

impl FetchBody {
    /// Reads the `body` property of a guest `fetch` init object.
    fn from_init<'js>(init: &BoundObject<'js>) -> Result<Option<Self>, Error> {
        // A `Uint8Array` is not an `Array`, so it must be read through the GuestJS byte
        // marshalling or serde would deserialize it as a map keyed by index.
        if let Ok(Some(bytes)) = init.get::<Option<Bytes>>("body") {
            return Ok(Some(Self::Bytes(bytes)));
        }

        Ok(
            init.get::<Option<String>>("body")?
                .map(Self::Text)
        )
    }
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
    pub(crate) body: Option<FetchBody>,
}

impl FetchRequest {
    pub(crate) fn from_args<'js>(scope: &Scope<'js>, args: &Args<'js>) -> Result<Self, Error> {
        let init = args.get_opt::<Object>(scope, 1)?;

        Ok(Self {
            url: args.get_owned::<String>(scope, 0)?,
            body: init
                .as_ref()
                .map(FetchBody::from_init)
                .transpose()?
                .flatten(),
            init: FetchInit {
                method: init
                    .as_ref()
                    .and_then(|init| init.get::<Option<String>>("method").ok())
                    .flatten(),
                headers: init
                    .as_ref()
                    .and_then(|init| init.get::<Option<FetchHeaders>>("headers").ok())
                    .flatten(),
            },
        })
    }
}
