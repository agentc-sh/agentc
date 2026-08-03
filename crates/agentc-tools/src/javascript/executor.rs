// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_agent::types::capability::{Capability, CapabilitySet};
use agentc_executor_typescript::{executor::ExecutorBuilder, guestjs::llrt::Llrt};

/// Extends a TypeScript executor builder with JavaScript tool capabilities.
pub trait ExecutorBuilderToolExt {
    /// Adds guest facilities granted by a JavaScript package's tool capabilities.
    fn with_tool_capabilities<I>(self, capabilities: I) -> Self
    where
        I: Into<CapabilitySet>;
}

impl ExecutorBuilderToolExt for ExecutorBuilder {
    fn with_tool_capabilities<I>(self, capabilities: I) -> Self
    where
        I: Into<CapabilitySet>,
    {
        let capabilities = capabilities.into();
        let network = capabilities.has(&Capability::from("network"));
        let filesystem = capabilities.has_any(&[
            Capability::from("filesystem::read"),
            Capability::from("filesystem::write"),
        ]);

        self.configure(move |builder| {
            builder.bind_native(match (network, filesystem) {
                (true, true) => Llrt::builder().fetch().fs().build(),
                (true, false) => Llrt::builder().fetch().build(),
                (false, true) => Llrt::builder().fs().build(),
                (false, false) => Llrt::builder().build(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use agentc_agent::types::capability::CapabilitySet;
    use agentc_executor_typescript::executor::Executor;

    use crate::javascript::executor::ExecutorBuilderToolExt;

    const CAPABILITY_SOURCE: &str = r#"
import { readFileSync } from "node:fs";

export default [
    typeof fetch,
    typeof readFileSync,
].join(":");
"#;

    struct TestExecutor;

    impl TestExecutor {
        async fn default_export<I>(capabilities: I) -> String
        where
            I: Into<CapabilitySet>,
        {
            let executor = Executor::builder("capabilities.ts", CAPABILITY_SOURCE)
                .workers(1)
                .with_tool_capabilities(capabilities)
                .build()
                .await
                .unwrap();
            let result = executor
                .execute(|context| {
                    Box::pin(async move {
                        context
                            .module()
                            .get::<String>("default")
                            .await
                    })
                })
                .await
                .unwrap();

            executor.shutdown().await.unwrap();

            result
        }
    }

    #[tokio::test]
    async fn grants_network_and_filesystem_capabilities() {
        assert_eq!(
            TestExecutor::default_export(["network", "filesystem::read",]).await,
            "function:function",
        );
        assert_eq!(
            TestExecutor::default_export(["network", "filesystem::write",]).await,
            "function:function",
        );
    }
}
