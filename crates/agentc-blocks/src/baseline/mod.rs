// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod a2a;
pub mod mcp;

use agentc_compiler::generator::{
    blocks::{fragment::FragmentBlock, BlockSet},
    extension::Contribution,
};

use crate::{
    baseline::{a2a::A2aAgentFragment, mcp::McpAgentFragment},
    composition::GenerationContribution,
    config::sections::{
        a2a::A2aSection, filesystem::FilesystemSection, mcp::McpSection, network::NetworkSection,
    },
    contributions::dependency::{CargoDependencies, CargoPatches},
    errors::BlocksError,
};

pub struct ResolvedBaseline {
    pub contribution: GenerationContribution,
    pub integrations: Vec<GenerationContribution>,
}

/// The capabilities every generated agent has, independent of graph, archetype and protocol.
pub struct Baseline;

impl Baseline {
    pub fn resolve() -> Result<ResolvedBaseline, BlocksError> {
        Ok(ResolvedBaseline {
            contribution: GenerationContribution::new().with_blocks(
                BlockSet::new()
                    .add(NetworkSection::block("baseline_network_section"))
                    .add(FilesystemSection::block("baseline_filesystem_section"))
                    .add(McpSection::block("baseline_mcp_section"))
                    .add(A2aSection::block("baseline_a2a_section"))
                    .add(
                        FragmentBlock::builder()
                            .id("baseline_mcp_agent")
                            .contribute(Contribution::<String>::strict("agent::use"))
                            .contribute(Contribution::<String>::strict("agent::tools"))
                            .contribute(Contribution::<CargoDependencies>::strict(
                                "cargo::dependencies",
                            ))
                            .contribute(Contribution::<CargoPatches>::strict("cargo::patches"))
                            .build(McpAgentFragment),
                    )
                    .add(
                        FragmentBlock::builder()
                            .id("baseline_a2a_agent")
                            .contribute(Contribution::<String>::strict("agent::use"))
                            .contribute(Contribution::<String>::strict("agent::tools"))
                            .contribute(Contribution::<CargoDependencies>::strict(
                                "cargo::dependencies",
                            ))
                            .contribute(Contribution::<CargoPatches>::strict("cargo::patches"))
                            .build(A2aAgentFragment),
                    )
                    .into_inner(),
            ),
            integrations: Vec::new(),
        })
    }
}
