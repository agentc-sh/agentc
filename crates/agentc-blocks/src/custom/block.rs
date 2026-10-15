// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::collections::BTreeMap;

use agentc_compiler::generator::{
    blocks::{Block, template::TemplateBlock},
    context::GenerationContext,
    errors::GeneratorError,
    extension::{
        Contribution, ErasedContribution, ErasedContributionValue, ExtensionRegistry,
        RenderedTokenStream,
    },
    vfs::VirtualFileSystem,
};

use crate::{
    context::{ResolvedContext, ResolvedContextBlock, ResolvedContextBlockContribution},
    contributions::dependency::{
        CargoDependencies, CargoDependencyContribution, ExternalDependencyContribution,
    },
};

enum CustomContributionValue {
    CargoDependencies(Vec<ExternalDependencyContribution>),
}

impl CustomContributionValue {
    fn erase(&self, point: &str) -> ErasedContribution {
        match self {
            Self::CargoDependencies(_) => Contribution::<CargoDependencies>::strict(point).erase(),
        }
    }

    fn value(&self) -> Result<ErasedContributionValue, GeneratorError> {
        match self {
            Self::CargoDependencies(dependencies) => CargoDependencies::from_entries(
                dependencies
                    .iter()
                    .cloned()
                    .map(CargoDependencyContribution::external),
            )
            .map(ErasedContributionValue::new)
            .map_err(|error| GeneratorError::unexpected(error.to_string())),
        }
    }
}

pub struct CustomBlock {
    template: TemplateBlock,
    values: BTreeMap<String, CustomContributionValue>,
}

impl From<&ResolvedContextBlock> for CustomBlock {
    fn from(block: &ResolvedContextBlock) -> Self {
        let mut template = TemplateBlock::builder().id(&block.name);
        let mut values = BTreeMap::new();

        for (path, file) in &block.generates {
            template = template.file(path, &file.content);
        }

        for (point, contribution) in &block.contributes {
            match contribution {
                ResolvedContextBlockContribution::Text(file) => {
                    template = template.contribute(Contribution::<String>::strict(point), &file.content);
                }
                ResolvedContextBlockContribution::Tokens(file) => {
                    template = template.contribute(
                        Contribution::<RenderedTokenStream>::strict(point),
                        &file.content,
                    );
                }
                ResolvedContextBlockContribution::CargoDependencies(dependencies) => {
                    values.insert(
                        point.clone(),
                        CustomContributionValue::CargoDependencies(
                            dependencies
                                .iter()
                                .map(ExternalDependencyContribution::from)
                                .collect(),
                        ),
                    );
                }
            }
        }

        Self {
            template: template.build(),
            values,
        }
    }
}

#[async_trait]
impl Block<ResolvedContext> for CustomBlock {
    fn id(&self) -> &str {
        Block::<ResolvedContext>::id(&self.template)
    }

    fn contributions(&self) -> Vec<ErasedContribution> {
        Block::<ResolvedContext>::contributions(&self.template)
            .into_iter()
            .chain(
                self.values
                    .iter()
                    .map(|(point, value)| value.erase(point)),
            )
            .collect()
    }

    async fn render_contribution(
        &self,
        ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError> {
        match self.values.get(point) {
            Some(value) => value.value(),
            None => {
                self.template
                    .render_contribution(ctx, point)
                    .await
            }
        }
    }

    async fn render(
        &self,
        ctx: &GenerationContext<ResolvedContext>,
        registry: &ExtensionRegistry,
        vfs: &mut VirtualFileSystem,
    ) -> Result<(), GeneratorError> {
        self.template
            .render(ctx, registry, vfs)
            .await
    }
}
