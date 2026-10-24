// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::marker::PhantomData;

use agentc_executor_typescript::{
    backend::ExecutorBackend,
    guestjs::{backend::Backend, errors::Error, host_class},
    json::Json,
};
use jsonschema::draft202012;
use serde_json::Value;

pub struct Schema<B: Backend> {
    value: Value,
    backend: PhantomData<fn() -> B>,
}

#[host_class(backend = B, crate_path = agentc_executor_typescript::guestjs)]
impl<B: ExecutorBackend> Schema<B> {
    #[guestjs(constructor)]
    fn new(schema: Json) -> Result<Self, Error> {
        draft202012::meta::validate(&schema.0)
            .map_err(|error| Error::unexpected(format!("invalid tool schema: {error}")))?;

        Ok(Self { value: schema.0, backend: PhantomData })
    }

    #[guestjs(get)]
    fn value(&self) -> Result<Json, Error> {
        Ok(Json(self.value.clone()))
    }

    pub(crate) fn document(&self) -> &Value {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use agentc_executor_typescript::{
        executor::Executor,
        guestjs::{bundle::Bundle, handle::protocol::ObjectProtocol, quickjs::QuickJs},
        json::Json,
    };

    use crate::javascript::bindings::executor::ExecutorBuilderToolsExt;

    const VALID_SOURCE: &str = r#"
import { Schema } from "agentc:tools";

export const parameters = new Schema({
    type: "object",
    properties: { city: { type: "string" } },
    required: ["city"],
});
"#;

    const INVALID_SOURCE: &str = r#"
import { Schema } from "agentc:tools";

export const parameters = new Schema({ type: "objct" });
"#;

    const UNKNOWN_META_SOURCE: &str = r#"
import { Schema } from "agentc:tools";

export const parameters = new Schema({
    $schema: "http://example.com/mine",
    type: "object",
});
"#;

    #[tokio::test]
    async fn a_valid_schema_constructs_and_reads_back() {
        let executor = Executor::<QuickJs>::builder("schema")
            .bundle(Bundle::single("schema.ts", VALID_SOURCE).unwrap())
            .workers(1)
            .standard_environment()
            .with_tools()
            .build()
            .await
            .expect("executor builds");

        assert_eq!(
            executor
                .execute(|context| Box::pin(async move {
                    context
                        .module()
                        .object("parameters")?
                        .get::<Json, _>("value")
                        .map(|value| value.0)
                }))
                .await
                .expect("guest call succeeds"),
            serde_json::json!({
                "type": "object",
                "properties": { "city": { "type": "string" } },
                "required": ["city"],
            }),
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn schema_rejects_invalid_document() {
        let error = match Executor::<QuickJs>::builder("schema")
            .bundle(Bundle::single("schema.ts", INVALID_SOURCE).unwrap())
            .workers(1)
            .standard_environment()
            .with_tools()
            .build()
            .await
        {
            Ok(_) => panic!("module evaluation succeeds"),
            Err(error) => error,
        };

        assert!(
            error
                .to_string()
                .contains("invalid tool schema"),
            "a malformed schema must fail at module evaluation; got: {error}",
        );
    }

    #[tokio::test]
    async fn an_unknown_meta_schema_does_not_panic() {
        Executor::<QuickJs>::builder("schema")
            .bundle(Bundle::single("schema.ts", UNKNOWN_META_SOURCE).unwrap())
            .workers(1)
            .standard_environment()
            .with_tools()
            .build()
            .await
            .expect("an unrecognised $schema is validated against the pinned draft")
            .shutdown()
            .await
            .expect("executor shuts down");
    }
}
