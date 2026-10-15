// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;

use crate::generator::{
    blocks::template::traits::RenderedContribution, errors::GeneratorError,
    extension::RenderedTokenStream,
};

impl RenderedContribution for String {
    fn from_rendered(rendered: String) -> Result<Self, GeneratorError> {
        Ok(rendered)
    }
}

impl RenderedContribution for RenderedTokenStream {
    fn from_rendered(rendered: String) -> Result<Self, GeneratorError> {
        rendered
            .parse::<TokenStream>()
            .map(Self::from)
            .map_err(|error| GeneratorError::unexpected(error.to_string()))
    }
}
