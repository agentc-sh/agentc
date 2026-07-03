// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::Serialize;
use std::ops::Deref;

#[derive(Debug, Clone, Serialize)]
pub struct GenerationContext<T: Serialize> {
    pub(crate) values: T,
}

impl<T: Serialize> GenerationContext<T> {
    pub fn new(values: T) -> Self {
        Self { values }
    }

    pub fn as_inner(&self) -> &T {
        &self.values
    }

    pub fn as_inner_mut(&mut self) -> &mut T {
        &mut self.values
    }

    pub fn into_inner(self) -> T {
        self.values
    }
}

impl<T> From<T> for GenerationContext<T>
where
    T: Serialize,
{
    fn from(values: T) -> Self {
        Self::new(values)
    }
}

impl<T> Deref for GenerationContext<T>
where
    T: Serialize,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Debug, Clone, Serialize, PartialEq)]
    struct Config {
        name: String,
        count: u32,
    }

    fn config() -> Config {
        Config { name: "test".into(), count: 42 }
    }

    #[test]
    fn new_and_as_inner_roundtrip() {
        let ctx = GenerationContext::new(config());
        assert_eq!(ctx.as_inner(), &config());
    }

    #[test]
    fn deref_exposes_inner_fields_directly() {
        let ctx = GenerationContext::new(config());
        // Deref lets us access fields without going through as_inner()
        assert_eq!(ctx.name, "test");
        assert_eq!(ctx.count, 42);
    }

    #[test]
    fn from_impl_wraps_value() {
        let ctx: GenerationContext<Config> = config().into();
        assert_eq!(ctx.as_inner(), &config());
    }

    #[test]
    fn into_inner_unwraps_value() {
        let ctx = GenerationContext::new(config());
        assert_eq!(ctx.into_inner(), config());
    }

    #[test]
    fn as_inner_mut_allows_mutation() {
        let mut ctx = GenerationContext::new(config());
        ctx.as_inner_mut().count = 99;
        assert_eq!(ctx.count, 99);
    }
}
