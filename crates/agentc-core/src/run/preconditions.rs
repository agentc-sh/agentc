// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::pipeline::steps::{
    compose::ComposeStepOutput,
    preflight::{Precondition, PreconditionError},
};

/// Requires that the composed toolchain can invoke what it builds.
pub struct RunSupported;

impl Precondition<ComposeStepOutput> for RunSupported {
    fn name(&self) -> &str {
        "run_supported"
    }

    fn verify(&self, value: &ComposeStepOutput) -> Result<(), PreconditionError> {
        value
            .toolchain
            .supports_run()
            .then_some(())
            .ok_or_else(|| {
                PreconditionError::new(
                    self.name(),
                    format!("archetype {:?} does not support run", value.archetype_name),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use serde_json::json;

    use agentc_blocks::context::ResolvedContext;
    use agentc_compiler::{
        compiler::{traits::OutputSink, types::CompileParams},
        runner::types::{RunOutcome, RunParams},
        toolchain::{errors::ToolchainError, traits::ErasedToolchain},
    };

    use super::*;

    struct FakeToolchain {
        supports_run: bool,
    }

    #[async_trait]
    impl ErasedToolchain for FakeToolchain {
        fn supports_run(&self) -> bool {
            self.supports_run
        }

        async fn compile_erased(
            &mut self,
            _params: CompileParams,
            _output_sink: &dyn OutputSink,
        ) -> Result<(), ToolchainError> {
            Ok(())
        }

        async fn run_erased(&self, _params: RunParams) -> Result<RunOutcome, ToolchainError> {
            Err(ToolchainError::RunUnsupported)
        }
    }

    fn composed(archetype_name: &str, supports_run: bool) -> ComposeStepOutput {
        ComposeStepOutput {
            agent_name: "assistant".to_string(),
            archetype_name: archetype_name.to_string(),
            graph_name: "react".to_string(),
            protocol_names: Vec::new(),
            context: serde_json::from_value::<ResolvedContext>(json!({
                "slug": "assistant",
                "agent_name": "assistant",
                "runtime": { "default_tenant_id": "default" },
                "providers": [],
                "agent": {
                    "version": "0.1.0",
                    "description": null,
                    "prompt": null,
                    "capabilities": null,
                    "capability_policy": null,
                    "model": { "provider": "anthropic", "name": "claude" }
                },
                "blocks": {},
                "tools": {},
                "skills": {},
                "http_server": null
            }))
            .unwrap(),
            toolchain: Box::new(FakeToolchain { supports_run }),
            blocks: Vec::new(),
            embedded_assets: Vec::new(),
            assets: Vec::new(),
        }
    }

    #[test]
    fn a_runnable_toolchain_passes() {
        assert!(
            RunSupported
                .verify(&composed("standalone", true))
                .is_ok()
        );
    }

    #[test]
    fn a_toolchain_that_cannot_run_fails_naming_the_archetype() {
        let error = RunSupported
            .verify(&composed("serverless", false))
            .unwrap_err();

        assert_eq!(error.name, "run_supported");
        assert_eq!(error.reason, "archetype \"serverless\" does not support run");
    }
}
