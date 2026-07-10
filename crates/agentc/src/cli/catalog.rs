// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_core::blocks::{
    archetype::standalone::StandaloneArchetype, catalog::CompilationCatalog, errors::BlocksError,
    graph::react::ReActGraph,
    protocol::{
        a2a::A2aProtocol,
        ag_ui::AgUiProtocol,
    },
};

pub struct DefaultCompilationCatalog;

impl DefaultCompilationCatalog {
    pub fn build() -> Result<CompilationCatalog, BlocksError> {
        CompilationCatalog::builder()
            .with_archetype(StandaloneArchetype)
            .with_graph(ReActGraph)
            .with_protocol(AgUiProtocol)
            .with_protocol(A2aProtocol)
            .build()
    }
}
