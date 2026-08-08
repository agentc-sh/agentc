// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use quote::quote;

use agentc_compiler::generator::{
    blocks::fragment::{Fragment, FragmentBlock},
    context::GenerationContext,
    errors::GeneratorError,
    extension::{Contribution, ErasedContributionValue},
};

use crate::{
    config::sections::{
        block::ConfigSectionBlockBuilderExt,
        contribution::{ConfigSectionContribution, ConfigSections},
    },
    context::ResolvedContext,
    contributions::dependency::{
        CargoDependencies, CargoDependencyContribution, ExternalDependencyContribution,
    },
};

pub struct TaskQueueSection;

impl TaskQueueSection {
    pub const NAME: &'static str = "task_queue";

    pub fn block(id: &'static str) -> FragmentBlock<ResolvedContext> {
        FragmentBlock::<ResolvedContext>::builder()
            .id(id)
            .contribute_config_sections()
            .contribute(Contribution::<CargoDependencies>::strict("cargo::dependencies"))
            .build(Self)
    }

    fn section(&self) -> Result<ConfigSections, GeneratorError> {
        ConfigSections::from_entries([ConfigSectionContribution::new(Self::NAME)
            .types(quote! {
                #[derive(Debug, Clone, Serialize, Deserialize)]
                #[serde(default)]
                pub struct TaskQueueConfig {
                    pub worker_count: usize,
                    pub max_queue_capacity: usize,
                    pub batch_size: usize,
                    pub batch_timeout_ms: usize,
                }

                impl Default for TaskQueueConfig {
                    fn default() -> Self {
                        TaskQueueConfig {
                            worker_count: 4,
                            max_queue_capacity: 256,
                            batch_size: 16,
                            batch_timeout_ms: 10,
                        }
                    }
                }
            })
            .fields(quote! {
                pub task_queue: TaskQueueConfig,
            })])
        .map_err(|error| GeneratorError::unexpected(error.to_string()))
    }
}

impl Fragment<ResolvedContext> for TaskQueueSection {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match point {
            "config::sections::use"
            | "config::sections::types"
            | "config::sections::fields"
            | "config::sections::loader"
            | "config::sections::mapper" => Ok(ErasedContributionValue::new(self.section()?)),
            "cargo::dependencies" => Ok(ErasedContributionValue::new(
                CargoDependencies::from_entries([CargoDependencyContribution::external(
                    ExternalDependencyContribution::new("jobq")
                        .git("https://github.com/wizrds/jobq-rs.git")
                        .version("0.3.1"),
                )])
                .map_err(|error| GeneratorError::unexpected(error.to_string()))?,
            )),
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    fn context() -> GenerationContext<ResolvedContext> {
        GenerationContext::new(
            serde_json::from_value(json!({
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
        )
    }

    #[test]
    fn the_section_defines_task_queue_config_and_its_field() {
        let sections = TaskQueueSection
            .generate_contribution(&context(), "config::sections::types")
            .unwrap()
            .downcast::<ConfigSections>()
            .unwrap();
        let section = sections
            .get(&TaskQueueSection::NAME)
            .expect("task queue section is contributed");

        assert!(
            section
                .types
                .as_str()
                .contains("pub struct TaskQueueConfig")
        );
        assert!(
            section
                .fields
                .as_str()
                .contains("pub task_queue : TaskQueueConfig")
        );
    }

    #[test]
    fn contributes_the_jobq_dependency() {
        let dependencies = TaskQueueSection
            .generate_contribution(&context(), "cargo::dependencies")
            .unwrap()
            .downcast::<CargoDependencies>()
            .unwrap();

        assert!(dependencies.get(&"jobq").is_some());
    }
}
