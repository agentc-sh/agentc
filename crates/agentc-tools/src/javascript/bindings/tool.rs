// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::future::Future;

use agentc_executor_typescript::guestjs::{errors::Error, host_class};

use crate::javascript::bindings::output::ToolOutput;

pub struct Tool;

#[host_class(crate_path = agentc_executor_typescript::guestjs)]
impl Tool {
    #[guestjs(constructor)]
    fn new() -> Result<Self, Error> {
        Ok(Self)
    }

    #[guestjs(async_method)]
    fn execute(&self) -> Result<impl Future<Output = Result<ToolOutput, Error>> + 'static, Error> {
        Ok(async { Err(Error::unexpected("a Tool subclass must implement execute")) })
    }
}

#[cfg(test)]
mod tests {
    use agentc_executor_typescript::{
        executor::Executor,
        guestjs::handle::{BoundClass, BoundConstructorProtocol, BoundObjectProtocol, Promise},
        json::Json,
    };

    use super::*;
    use crate::javascript::bindings::executor::ExecutorBuilderToolsExt;

    const TOOL_SOURCE: &str = r#"
import { Tool } from "agentc:tools";

export class Weather extends Tool<{ city: string }, { conditions: string }> {
    static readonly description = "Reports the weather.";

    async execute() {
        return { output: { conditions: "sunny" } };
    }
}

export class Silent extends Tool {
}

export class NotATool {
    async execute() {
        return { output: null };
    }
}

export { Tool };
"#;

    async fn executor() -> Executor {
        Executor::builder("tool.ts", TOOL_SOURCE)
            .workers(1)
            .standard_environment()
            .with_tools()
            .build()
            .await
            .expect("executor builds")
    }

    #[tokio::test]
    async fn a_guest_class_extends_the_host_tool_class() {
        let executor = executor().await;

        executor
            .execute(|context| {
                Box::pin(async move {
                    context
                        .guest()
                        .scope(async move |scope| {
                            let base = BoundClass::of::<Tool>(&scope)?;
                            let module = context.module().bind(&scope)?;

                            assert!(
                                module
                                    .class_as::<Tool>("Weather")?
                                    .is_subclass_of(&base)?,
                                "a generic `extends Tool<A, B>` transpiles to a plain \
                             `extends Tool`, and `BoundClass::of` resolves the same \
                             constructor `agentc:tools` exports",
                            );

                            assert_eq!(
                                module
                                    .class_as::<Tool>("Weather")?
                                    .get::<String>("description")?,
                                "Reports the weather.",
                                "`static readonly` survives the oxc transform",
                            );

                            assert!(
                                !module
                                    .class_as::<Tool>("NotATool")?
                                    .is_subclass_of(&base)?,
                                "an unrelated class is not a subclass",
                            );

                            assert!(
                                !module
                                    .class_as::<Tool>("Tool")?
                                    .is_subclass_of(&base)?,
                                "`is_subclass_of` is strict, so the host class is not a \
                             subclass of itself and cannot be exported as a tool",
                            );

                            Ok(())
                        })
                        .await
                })
            })
            .await
            .expect("guest call succeeds");

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn tool_without_execute_reports_a_clear_error() {
        let executor = executor().await;

        let error = match executor
            .execute(|context| {
                Box::pin(async move {
                    context
                        .guest()
                        .scope(async move |scope| {
                            context
                                .module()
                                .bind(&scope)?
                                .class_as::<Tool>("Silent")?
                                .construct(())?
                                .call_method::<_, Promise<Json>>("execute", ())?
                                .await
                        })
                        .await
                })
            })
            .await
        {
            Ok(_) => panic!("the base implementation resolves"),
            Err(error) => error,
        };

        assert!(
            error
                .to_string()
                .contains("a Tool subclass must implement execute"),
            "the base `execute` must reject inside the promise, not throw synchronously; \
             got: {error}",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }
}
