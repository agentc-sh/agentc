// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::generator::errors::GeneratorError;

pub trait RenderedContribution: Send + Sync + Sized + 'static {
    fn from_rendered(rendered: String) -> Result<Self, GeneratorError>;
}
