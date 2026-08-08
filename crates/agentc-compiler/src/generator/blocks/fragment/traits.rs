// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::Serialize;

use crate::generator::{
    context::GenerationContext, errors::GeneratorError, extension::ErasedContributionValue,
};

pub trait Fragment<T>: Send + Sync
where
    T: Serialize + Send + Sync,
{
    fn generate_contribution(
        &self,
        ctx: &GenerationContext<T>,
        point: &str,
    ) -> Result<ErasedContributionValue, GeneratorError>;
}
