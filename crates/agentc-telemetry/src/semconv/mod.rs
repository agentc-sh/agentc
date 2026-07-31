// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod genai;

pub use opentelemetry_semantic_conventions::*;

/// Exact input accepted by an agent invocation.
pub const AGENTC_AGENT_INPUT: &str = "agentc.agent.input";

/// Exact outcome returned by an agent invocation.
pub const AGENTC_AGENT_OUTPUT: &str = "agentc.agent.output";

/// `gen_ai.execute_tool` operation duration metric name.
pub const GEN_AI_EXECUTE_TOOL_DURATION: &str = "gen_ai.execute_tool.duration";

/// `gen_ai.invoke_agent` operation duration metric name.
pub const GEN_AI_INVOKE_AGENT_DURATION: &str = "gen_ai.invoke_agent.duration";

/// `gen_ai.invoke_workflow` operation duration metric name.
pub const GEN_AI_WORKFLOW_DURATION: &str = "gen_ai.workflow.duration";
