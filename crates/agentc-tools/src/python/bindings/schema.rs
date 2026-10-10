// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_python::{
    guestpy::{errors::Error, host_class},
    json::Json,
};
use jsonschema::draft202012;
use serde_json::Value;

pub struct Schema {
    value: Value,
}

#[host_class(crate_path = agentc_executor_python::guestpy)]
impl Schema {
    #[guestpy(constructor)]
    fn new(schema: Json) -> Result<Self, Error> {
        draft202012::meta::validate(&schema.0)
            .map_err(|error| Error::conversion(format!("invalid tool schema: {error}")))?;

        Ok(Self { value: schema.0 })
    }

    #[guestpy(get)]
    fn value(&self) -> Result<Json, Error> {
        Ok(Json(self.value.clone()))
    }

    pub(crate) fn document(&self) -> &Value {
        &self.value
    }
}
