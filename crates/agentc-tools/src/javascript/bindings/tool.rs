// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{future::Future, marker::PhantomData};

use agentc_executor_typescript::{
    backend::ExecutorBackend,
    guestjs::{backend::Backend, errors::Error, host_class},
};

use crate::javascript::bindings::output::ToolOutput;

pub struct Tool<B: Backend> {
    backend: PhantomData<fn() -> B>,
}

#[host_class(backend = B, crate_path = agentc_executor_typescript::guestjs)]
impl<B: ExecutorBackend> Tool<B> {
    #[guestjs(constructor)]
    fn new() -> Result<Self, Error> {
        Ok(Self { backend: PhantomData })
    }

    #[guestjs(async_method)]
    fn execute(
        &self,
    ) -> Result<impl Future<Output = Result<ToolOutput, Error>> + use<B> + 'static, Error> {
        Ok(async { Err(Error::unexpected("a Tool subclass must implement execute")) })
    }
}

#[cfg(test)]
mod tests {
    use agentc_executor_typescript::{
        executor::Executor,
        guestjs::{
            bundle::Bundle,
            handle::{
                promise::Promise,
                protocol::ObjectProtocol,
            },
            host::class::HostClass,
            quickjs::QuickJs,
        },
        json::Json,
    };

    use super::*;
    use crate::javascript::bindings::{executor::ExecutorBuilderToolsExt, guest::GuestToolClass};

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

    async fn executor() -> Executor<QuickJs> {
        Executor::<QuickJs>::builder("tool")
            .bundle(Bundle::single("tool.ts", TOOL_SOURCE).unwrap())
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
                    let module = context.module();

                    assert!(
                        module
                            .get::<GuestToolClass<QuickJs>, _>("Weather")
                            .is_ok(),
                        "a generic `extends Tool<A, B>` transpiles to a plain `extends Tool`, \
                         and `Tool::class` resolves the same constructor `agentc:tools` exports",
                    );

                    assert_eq!(
                        module
                            .class("Weather")?
                            .get::<String, _>("description")?,
                        "Reports the weather.",
                        "`static readonly` survives the oxc transform",
                    );

                    assert!(
                        !module
                            .get::<GuestToolClass<QuickJs>, _>("NotATool")
                            .is_ok(),
                        "an unrelated class is not a subclass",
                    );

                    assert!(
                        !module
                            .get::<GuestToolClass<QuickJs>, _>("Tool")
                            .is_ok(),
                        "`is_subclass_of` is strict, so the host class is not a subclass of \
                         itself and cannot be exported as a tool",
                    );

                    Ok(())
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
                        .module()
                        .class("Silent")?
                        .construct(())?
                        .call_method::<_, _, Promise<QuickJs, Json>>("execute", ())?
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
