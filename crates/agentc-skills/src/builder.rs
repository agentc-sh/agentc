// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use agentc_agent::{
    agent::AgentBuilder,
    context::AgentContext,
    graph::state::{GraphNode, GraphState, StateOf},
    instrument::InvokeAgentSpans,
    tools::registry::ToolRegistryBuilder,
    types::event::AgentEvent,
};

use crate::{
    registry::SkillRegistry,
    tools::{
        describe::DescribeSkillTool,
        get::GetSkillTool,
        list::ListSkillsTool,
        read::ReadSkillFileTool,
        run::{MaterializationPolicy, RunSkillScriptTool},
    },
};

/// Extension methods on [`ToolRegistryBuilder`](agentc_agent::tools::registry::ToolRegistryBuilder)
/// for registering skill tools.
pub trait ToolRegistryBuilderSkillsExt: Sized {
    /// Register all skill tools from the given [`SkillRegistry`].
    ///
    /// The registry is wrapped in an [`Arc`] internally. Use
    /// [`with_skill_registry_arc`](ToolRegistryBuilderSkillsExt::with_skill_registry_arc)
    /// when you already hold an [`Arc`] and want to avoid an extra allocation.
    ///
    /// If the registry is empty no tools are registered and the builder is
    /// returned unchanged.
    fn with_skill_registry<S: GraphState + 'static>(
        self,
        registry: SkillRegistry,
        policy: MaterializationPolicy,
    ) -> Self {
        self.with_skill_registry_arc::<S>(Arc::new(registry), policy)
    }

    /// Register all skill tools from the given [`Arc<SkillRegistry>`].
    ///
    /// If the registry is empty no tools are registered and the builder is
    /// returned unchanged.
    fn with_skill_registry_arc<S: GraphState + 'static>(
        self,
        registry: Arc<SkillRegistry>,
        policy: MaterializationPolicy,
    ) -> Self;
}

impl ToolRegistryBuilderSkillsExt for ToolRegistryBuilder {
    fn with_skill_registry_arc<S: GraphState + 'static>(
        self,
        registry: Arc<SkillRegistry>,
        policy: MaterializationPolicy,
    ) -> Self {
        if registry.is_empty() {
            return self;
        }

        self.with_typed_tool::<S, _>(ListSkillsTool { registry: registry.clone() })
            .with_typed_tool::<S, _>(GetSkillTool { registry: registry.clone() })
            .with_typed_tool::<S, _>(DescribeSkillTool { registry: registry.clone() })
            .with_typed_tool::<S, _>(ReadSkillFileTool { registry: registry.clone() })
            .with_typed_tool::<S, _>(RunSkillScriptTool::new(registry, policy))
    }
}

/// Extension methods on [`AgentBuilder`](agentc_agent::agent::AgentBuilder) for
/// registering skill tools and the prompt template vars contributor.
pub trait AgentBuilderSkillsExt: Sized {
    /// Register all skill tools and the [`TemplateVars`](agentc_prompt::vars::TemplateVars)
    /// contributor from the given [`SkillRegistry`].
    ///
    /// The registry is wrapped in an [`Arc`] internally. Use
    /// [`with_skill_registry_arc`](AgentBuilderSkillsExt::with_skill_registry_arc)
    /// when you already hold an [`Arc`] and want to avoid an extra allocation.
    ///
    /// If the registry is empty, no tools are registered and no contributor is
    /// added. The builder is returned unchanged in that case.
    fn with_skill_registry(self, registry: SkillRegistry, policy: MaterializationPolicy) -> Self {
        self.with_skill_registry_arc(Arc::new(registry), policy)
    }

    /// Register all skill tools and the [`TemplateVars`](agentc_prompt::vars::TemplateVars)
    /// contributor from the given [`Arc<SkillRegistry>`].
    ///
    /// If the registry is empty, no tools are registered and no contributor is
    /// added. The builder is returned unchanged in that case.
    fn with_skill_registry_arc(
        self,
        registry: Arc<SkillRegistry>,
        policy: MaterializationPolicy,
    ) -> Self;
}

impl<N, E, M> AgentBuilderSkillsExt for AgentBuilder<N, E, M>
where
    N: GraphNode<Context = AgentContext<E, M>> + InvokeAgentSpans + 'static,
    E: From<AgentEvent<StateOf<N>>> + Send + Clone + 'static,
    M: Send + Clone + 'static,
{
    fn with_skill_registry_arc(
        self,
        registry: Arc<SkillRegistry>,
        policy: MaterializationPolicy,
    ) -> Self {
        if registry.is_empty() {
            return self;
        }

        self.with_tool_registry(
            ToolRegistryBuilder::new()
                .with_skill_registry_arc::<StateOf<N>>(registry.clone(), policy)
                .build(),
        )
        .with_template_vars(registry)
    }
}
