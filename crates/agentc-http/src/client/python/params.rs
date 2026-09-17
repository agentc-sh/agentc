// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_python::guestpy::{
    FromGuest,
    marshal::collections::{Iterable, Mapping},
};

#[derive(FromGuest)]
#[guestpy(union, crate_path = agentc_executor_python::guestpy)]
pub enum Primitive {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Null(()),
}

impl Primitive {
    pub(crate) fn encode(self) -> String {
        match self {
            Self::Bool(true) => "true".to_owned(),
            Self::Bool(false) => "false".to_owned(),
            Self::Int(value) => value.to_string(),
            Self::Float(value) => value.to_string(),
            Self::Text(value) => value,
            Self::Null(()) => String::new(),
        }
    }
}

#[derive(FromGuest)]
#[guestpy(union, crate_path = agentc_executor_python::guestpy)]
pub enum ParamValues {
    One(Primitive),
    Many(Iterable<Vec<Primitive>>),
}

#[derive(FromGuest)]
#[guestpy(union, crate_path = agentc_executor_python::guestpy)]
pub enum QueryParams {
    Mapping(Mapping<String, ParamValues>),
    Pairs(Iterable<Vec<(String, Primitive)>>),
}

impl QueryParams {
    pub(crate) fn pairs(self) -> Vec<(String, String)> {
        match self {
            Self::Mapping(Mapping(entries)) => entries
                .into_iter()
                .flat_map(|(name, values)| match values {
                    ParamValues::One(value) => vec![(name, value.encode())],
                    ParamValues::Many(values) => values
                        .into_inner()
                        .into_iter()
                        .map(|value| (name.clone(), value.encode()))
                        .collect(),
                })
                .collect(),
            Self::Pairs(pairs) => pairs
                .into_inner()
                .into_iter()
                .map(|(name, value)| (name, value.encode()))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use agentc_executor_python::{
        executor::Executor,
        guestpy::{
            bundle::Bundle,
            errors::Error,
            handle::ObjectProtocol,
            host::{library::HostLibrary, module::ModuleSpec},
            marshal::FromGuest,
            rustpython::RustPython,
        },
    };

    use super::QueryParams;

    const SOURCE: &str = r#"
from agentc_http import encode_params


def inspect_params():
    return [
        encode_params({"a": "x", "b": 1, "c": 1.5, "d": True, "e": False, "f": None}),
        encode_params({"a": ["x", "y"], "b": "xy"}),
        encode_params([("a", "x"), ("a", "y"), ("b", 2)]),
    ]
"#;

    fn module() -> Result<ModuleSpec<RustPython>, Error> {
        Ok(ModuleSpec::new("agentc_http").function("encode_params", |enter, args| {
            let params = args.required::<QueryParams>(enter, 0, "params")?;

            args.finish()?;

            Ok(params.pairs())
        }))
    }

    async fn executor() -> Executor<RustPython> {
        Executor::<RustPython>::builder("agentc_http_unit_test")
            .bundle(Bundle::single("agentc_http_unit_test", SOURCE).expect("the bundle builds"))
            .workers(1)
            .configure(|runtime| Ok(runtime.bind(HostLibrary::new().with(module()?))))
            .build()
            .await
            .expect("executor builds")
    }

    async fn call<T>(executor: &Executor<RustPython>, export: &'static str) -> T::Owned
    where
        T: FromGuest<RustPython>,
        T::Owned: Send + 'static,
    {
        executor
            .execute(move |context| {
                Box::pin(async move {
                    context
                        .module()
                        .function(export)?
                        .call::<_, T>(())
                })
            })
            .await
            .expect("guest call succeeds")
    }

    #[tokio::test]
    async fn query_params_encode_primitives() {
        let executor = executor().await;

        assert_eq!(
            call::<agentc_executor_python::json::Json>(&executor, "inspect_params")
                .await
                .into_inner(),
            serde_json::json!([
                [
                    ["a", "x"],
                    ["b", "1"],
                    ["c", "1.5"],
                    ["d", "true"],
                    ["e", "false"],
                    ["f", ""]
                ],
                [["a", "x"], ["a", "y"], ["b", "xy"]],
                [["a", "x"], ["a", "y"], ["b", "2"]],
            ]),
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }
}
