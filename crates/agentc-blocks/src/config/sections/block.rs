// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_compiler::generator::{
    blocks::{codegen::block::CodeGenBlockBuilder, fragment::block::FragmentBlockBuilder},
    extension::Contribution,
};

use crate::{config::sections::contribution::ConfigSections, context::ResolvedContext};

pub trait ConfigSectionBlockBuilderExt {
    fn contribute_config_sections(self) -> Self;
}

impl ConfigSectionBlockBuilderExt for FragmentBlockBuilder<ResolvedContext> {
    fn contribute_config_sections(self) -> Self {
        self.contribute(Contribution::<ConfigSections>::lenient("config::sections::use"))
            .contribute(Contribution::<ConfigSections>::lenient("config::sections::types"))
            .contribute(Contribution::<ConfigSections>::lenient("config::sections::fields"))
            .contribute(Contribution::<ConfigSections>::lenient("config::sections::loader"))
            .contribute(Contribution::<ConfigSections>::lenient("config::sections::mapper"))
    }
}

impl ConfigSectionBlockBuilderExt for CodeGenBlockBuilder<ResolvedContext> {
    fn contribute_config_sections(self) -> Self {
        self.contribute(Contribution::<ConfigSections>::lenient("config::sections::use"))
            .contribute(Contribution::<ConfigSections>::lenient("config::sections::types"))
            .contribute(Contribution::<ConfigSections>::lenient("config::sections::fields"))
            .contribute(Contribution::<ConfigSections>::lenient("config::sections::loader"))
            .contribute(Contribution::<ConfigSections>::lenient("config::sections::mapper"))
    }
}
