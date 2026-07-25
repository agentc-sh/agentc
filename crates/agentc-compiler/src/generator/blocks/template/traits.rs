// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

use serde::Serialize;

use crate::generator::{
    context::GenerationContext,
    errors::GeneratorError,
    extension::{ErasedContributionValue, ExtensionRegistry},
};

pub trait TemplateFragment<T>: Send + Sync
where
    T: Serialize + Send + Sync,
{
    fn generate_files(
        &self,
        _ctx: &GenerationContext<T>,
        _registry: &ExtensionRegistry,
    ) -> Result<Vec<(PathBuf, String)>, GeneratorError> {
        Ok(vec![])
    }

    fn generate_contribution(
        &self,
        ctx: &GenerationContext<T>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError>;
}
