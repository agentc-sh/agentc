// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_compiler::generator::extension::RenderedTokenStream;
use proc_macro2::TokenStream;

use crate::{
    contributions::set::{ContributionSet, Mergeable},
    errors::BlocksError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSectionSlot {
    Use,
    Types,
    Fields,
    Loader,
    Mapper,
}

/// A named group of hand-written config code that is emitted exactly once no matter
/// how many components ask for it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigSectionContribution {
    pub name: &'static str,
    pub uses: RenderedTokenStream,
    pub types: RenderedTokenStream,
    pub fields: RenderedTokenStream,
    pub loader: RenderedTokenStream,
    pub mapper: RenderedTokenStream,
}

impl ConfigSectionContribution {
    pub fn new(name: &'static str) -> Self {
        Self { name, ..Default::default() }
    }

    pub fn uses(mut self, tokens: TokenStream) -> Self {
        self.uses = tokens.into();
        self
    }

    pub fn types(mut self, tokens: TokenStream) -> Self {
        self.types = tokens.into();
        self
    }

    pub fn fields(mut self, tokens: TokenStream) -> Self {
        self.fields = tokens.into();
        self
    }

    pub fn loader(mut self, tokens: TokenStream) -> Self {
        self.loader = tokens.into();
        self
    }

    pub fn mapper(mut self, tokens: TokenStream) -> Self {
        self.mapper = tokens.into();
        self
    }

    pub fn slot(&self, slot: ConfigSectionSlot) -> &RenderedTokenStream {
        match slot {
            ConfigSectionSlot::Use => &self.uses,
            ConfigSectionSlot::Types => &self.types,
            ConfigSectionSlot::Fields => &self.fields,
            ConfigSectionSlot::Loader => &self.loader,
            ConfigSectionSlot::Mapper => &self.mapper,
        }
    }
}

impl Mergeable for ConfigSectionContribution {
    type Key = &'static str;

    fn key(&self) -> Self::Key {
        self.name
    }

    fn merge(&mut self, other: Self) -> Result<(), BlocksError> {
        *self = other;

        Ok(())
    }
}

pub type ConfigSections = ContributionSet<ConfigSectionContribution>;
