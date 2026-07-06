// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::ops::Deref;

use crate::{
    fields::types::{FieldValue, IntoTypeTokens},
    types::RuntimeValue,
};

pub trait IntoFieldSpecs {
    fn extend_fields(&self, fields: &mut FieldsSpec);
}

#[derive(Debug, Clone)]
pub struct FieldSpec {
    /// Segments describing where this field lives, e.g. ["provider", "anthropic", "api_key"].
    /// The last segment is the field name; preceding segments define the nested struct path.
    pub path: Vec<String>,
    /// Type-erased value metadata derived from the RuntimeValue.
    pub value: FieldValue,
    /// Token stream for the Rust type of this field, e.g. `|| { quote! { String } }`.
    pub rust_type: fn() -> TokenStream,
}

impl FieldSpec {
    pub fn new<T>(path: &[&str], value: &RuntimeValue<T>) -> Self
    where
        T: serde::Serialize + IntoTypeTokens,
    {
        Self {
            path: path
                .iter()
                .map(|s| s.to_string())
                .collect(),
            value: FieldValue::from(value),
            rust_type: T::type_tokens,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FieldsSpec(pub Vec<FieldSpec>);

impl FieldsSpec {
    pub fn new(fields: Vec<FieldSpec>) -> Self {
        Self(fields)
    }

    pub fn push<T>(&mut self, path: &[&str], value: &RuntimeValue<T>)
    where
        T: serde::Serialize + IntoTypeTokens,
    {
        self.0.push(FieldSpec::new(path, value));
    }

    pub fn extend_from<S: IntoFieldSpecs>(&mut self, source: &S) {
        source.extend_fields(self);
    }

    /// Collect all field specs from `source` into a new `FieldsSpec`.
    pub fn collect_from<S: IntoFieldSpecs>(source: &S) -> Self {
        let mut fields = Self::new(vec![]);
        source.extend_fields(&mut fields);
        fields
    }

    pub fn as_inner(&self) -> &[FieldSpec] {
        &self.0
    }

    pub fn as_inner_mut(&mut self) -> &mut [FieldSpec] {
        &mut self.0
    }

    pub fn into_inner(self) -> Vec<FieldSpec> {
        self.0
    }

    pub fn get(&self, path: &[impl AsRef<str>]) -> Option<&FieldSpec> {
        self.as_inner().iter().find(|f| {
            f.path.len() == path.len()
                && f.path
                    .iter()
                    .zip(path.iter())
                    .all(|(a, b)| a == b.as_ref())
        })
    }

    /// Builds a `config.<segment>...` accessor expression for the field registered
    /// at `path`, or `None` when no field lives there. Code generators use this to
    /// read resolved values out of the generated `Config` struct.
    pub fn config_accessor(&self, path: &[&str]) -> Option<TokenStream> {
        self.get(path).map(|field| {
            field
                .path
                .iter()
                .fold(quote! { config }, |acc, segment| {
                    let ident = Ident::new(segment, Span::call_site());
                    quote! { #acc.#ident }
                })
        })
    }
}

impl From<Vec<FieldSpec>> for FieldsSpec {
    fn from(fields: Vec<FieldSpec>) -> Self {
        Self::new(fields)
    }
}

impl Deref for FieldsSpec {
    type Target = [FieldSpec];

    fn deref(&self) -> &Self::Target {
        self.as_inner()
    }
}

impl IntoIterator for FieldsSpec {
    type Item = FieldSpec;
    type IntoIter = std::vec::IntoIter<FieldSpec>;

    fn into_iter(self) -> Self::IntoIter {
        self.into_inner().into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_matches_only_the_exact_path() {
        let mut fields = FieldsSpec::new(vec![]);
        fields.push(&["a", "b"], &RuntimeValue::constant("x".to_string()));

        assert!(fields.get(&["a", "b"]).is_some());
        assert!(fields.get(&["a"]).is_none());
        assert!(fields.get(&["a", "b", "c"]).is_none());
    }

    #[test]
    fn config_accessor_builds_a_nested_field_expression() {
        let mut fields = FieldsSpec::new(vec![]);
        fields.push(&["server", "host"], &RuntimeValue::constant("h".to_string()));

        let accessor = fields
            .config_accessor(&["server", "host"])
            .expect("field is registered");

        assert_eq!(accessor.to_string().replace(' ', ""), "config.server.host");
        assert!(
            fields
                .config_accessor(&["missing"])
                .is_none()
        );
    }

    #[test]
    fn collect_from_gathers_fields_from_the_source() {
        struct Source;

        impl IntoFieldSpecs for Source {
            fn extend_fields(&self, fields: &mut FieldsSpec) {
                fields.push(&["one"], &RuntimeValue::constant(1u64));
                fields.push(&["two"], &RuntimeValue::constant(2u64));
            }
        }

        let fields = FieldsSpec::collect_from(&Source);

        assert_eq!(fields.as_inner().len(), 2);
        assert!(fields.get(&["one"]).is_some());
        assert!(fields.get(&["two"]).is_some());
    }
}
