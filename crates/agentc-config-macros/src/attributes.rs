// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::iter::once;

use darling::FromMeta;
use syn::{Attribute, Expr, ExprLit, Ident, Lit, Meta, Token, punctuated::Punctuated};

use crate::naming::{Naming, RenameRule};

struct SerdeMeta(Meta);

impl SerdeMeta {
    fn parse_all(attributes: &[Attribute]) -> Result<Vec<Self>, darling::Error> {
        let mut metas = Vec::new();

        for attribute in attributes
            .iter()
            .filter(|attribute| attribute.path().is_ident("serde"))
        {
            metas.extend(
                attribute
                    .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?
                    .into_iter()
                    .map(Self),
            );
        }

        Ok(metas)
    }

    fn is(&self, name: &str) -> bool {
        self.0.path().is_ident(name)
    }

    fn deserialize_name(&self) -> Result<Option<String>, darling::Error> {
        match &self.0 {
            Meta::NameValue(meta) => match &meta.value {
                Expr::Lit(ExprLit { lit: Lit::Str(name), .. }) => Ok(Some(name.value())),
                other => Err(darling::Error::unexpected_expr_type(other)),
            },
            Meta::List(list) => list
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?
                .into_iter()
                .map(Self)
                .find(|meta| meta.is("deserialize"))
                .map_or(Ok(None), |meta| meta.deserialize_name()),
            Meta::Path(path) => Err(darling::Error::custom("expected a name").with_span(path)),
        }
    }

    fn rename_rule(&self) -> Result<Option<RenameRule>, darling::Error> {
        self.deserialize_name()?
            .map(|rule| RenameRule::from_string(&rule))
            .transpose()
    }
}

pub(crate) struct SerdeContainer {
    rename_all: Option<RenameRule>,
    rename_all_fields: Option<RenameRule>,
    tag: Option<String>,
    content: Option<String>,
    untagged: bool,
    transparent: bool,
}

impl SerdeContainer {
    pub(crate) fn new(attributes: &[Attribute]) -> Result<Self, darling::Error> {
        let mut container = Self {
            rename_all: None,
            rename_all_fields: None,
            tag: None,
            content: None,
            untagged: false,
            transparent: false,
        };

        for meta in SerdeMeta::parse_all(attributes)? {
            if meta.is("rename_all") {
                container.rename_all = meta
                    .rename_rule()?
                    .or(container.rename_all);
            } else if meta.is("rename_all_fields") {
                container.rename_all_fields = meta
                    .rename_rule()?
                    .or(container.rename_all_fields);
            } else if meta.is("tag") {
                container.tag = meta.deserialize_name()?;
            } else if meta.is("content") {
                container.content = meta.deserialize_name()?;
            } else if meta.is("untagged") {
                container.untagged = true;
            } else if meta.is("transparent") {
                container.transparent = true;
            }
        }

        Ok(container)
    }

    pub(crate) fn rename_all(&self) -> Option<RenameRule> {
        self.rename_all
    }

    pub(crate) fn rename_all_fields(&self) -> Option<RenameRule> {
        self.rename_all_fields
    }

    pub(crate) fn tag(&self) -> Option<String> {
        self.tag.clone()
    }

    pub(crate) fn content(&self) -> Option<String> {
        self.content.clone()
    }

    pub(crate) fn is_untagged(&self) -> bool {
        self.untagged
    }

    pub(crate) fn is_transparent(&self) -> bool {
        self.transparent
    }
}

pub(crate) struct SerdeVariant {
    rename: Option<String>,
    aliases: Vec<String>,
    rename_all: Option<RenameRule>,
    skip: bool,
    untagged: bool,
}

impl SerdeVariant {
    pub(crate) fn new(attributes: &[Attribute]) -> Result<Self, darling::Error> {
        let mut variant = Self {
            rename: None,
            aliases: Vec::new(),
            rename_all: None,
            skip: false,
            untagged: false,
        };

        for meta in SerdeMeta::parse_all(attributes)? {
            if meta.is("rename") {
                variant.rename = meta
                    .deserialize_name()?
                    .or(variant.rename);
            } else if meta.is("alias") {
                variant
                    .aliases
                    .extend(meta.deserialize_name()?);
            } else if meta.is("rename_all") {
                variant.rename_all = meta
                    .rename_rule()?
                    .or(variant.rename_all);
            } else if meta.is("skip") || meta.is("skip_deserializing") || meta.is("other") {
                variant.skip = true;
            } else if meta.is("untagged") {
                variant.untagged = true;
            }
        }

        Ok(variant)
    }

    pub(crate) fn names(&self, ident: &Ident, rename_all: Option<RenameRule>) -> Vec<String> {
        once(Naming::member(ident, self.rename.clone(), rename_all))
            .chain(self.aliases.iter().cloned())
            .collect()
    }

    pub(crate) fn rename_all(&self) -> Option<RenameRule> {
        self.rename_all
    }

    pub(crate) fn is_skipped(&self) -> bool {
        self.skip
    }

    pub(crate) fn is_untagged(&self) -> bool {
        self.untagged
    }
}

pub(crate) struct SerdeField {
    rename: Option<String>,
    flatten: bool,
    skip: bool,
}

impl SerdeField {
    pub(crate) fn new(attributes: &[Attribute]) -> Result<Self, darling::Error> {
        let mut field = Self {
            rename: None,
            flatten: false,
            skip: false,
        };

        for meta in SerdeMeta::parse_all(attributes)? {
            if meta.is("rename") {
                field.rename = meta
                    .deserialize_name()?
                    .or(field.rename);
            } else if meta.is("flatten") {
                field.flatten = true;
            } else if meta.is("skip") || meta.is("skip_deserializing") {
                field.skip = true;
            }
        }

        Ok(field)
    }

    pub(crate) fn rename(&self) -> Option<String> {
        self.rename.clone()
    }

    pub(crate) fn is_flatten(&self) -> bool {
        self.flatten
    }

    pub(crate) fn is_skipped(&self) -> bool {
        self.skip
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::parse_quote;

    use super::{SerdeContainer, SerdeField, SerdeVariant};

    #[test]
    fn reads_field_rename() {
        assert_eq!(
            SerdeField::new(&[parse_quote!(#[serde(rename = "x")])])
                .unwrap()
                .rename(),
            Some(String::from("x")),
        );
    }

    #[test]
    fn reads_field_rename_for_deserialize() {
        assert_eq!(
            SerdeField::new(&[parse_quote!(#[serde(rename(deserialize = "x"))])])
                .unwrap()
                .rename(),
            Some(String::from("x")),
        );
    }

    #[test]
    fn ignores_field_rename_for_serialize_only() {
        assert_eq!(
            SerdeField::new(&[parse_quote!(#[serde(rename(serialize = "x"))])])
                .unwrap()
                .rename(),
            None,
        );
    }

    #[test]
    fn reads_field_flatten() {
        assert!(
            SerdeField::new(&[parse_quote!(#[serde(flatten)])])
                .unwrap()
                .is_flatten()
        );
    }

    #[test]
    fn reads_field_skip_and_skip_deserializing() {
        for skip in [quote!(skip), quote!(skip_deserializing)] {
            assert!(
                SerdeField::new(&[parse_quote!(#[serde(#skip)])])
                    .unwrap()
                    .is_skipped()
            );
        }
    }

    #[test]
    fn ignores_other_serde_keys() {
        let field = SerdeField::new(&[
            parse_quote!(#[serde(default, with = "m", bound(deserialize = "T: Default"))]),
        ])
        .unwrap();

        assert_eq!(field.rename(), None);
        assert!(!field.is_flatten());
        assert!(!field.is_skipped());
    }

    #[test]
    fn reads_container_rename_all_in_both_forms() {
        for rename_all in [
            quote!(rename_all = "camelCase"),
            quote!(rename_all(deserialize = "camelCase")),
        ] {
            assert_eq!(
                SerdeContainer::new(&[parse_quote!(#[serde(#rename_all)])])
                    .unwrap()
                    .rename_all()
                    .map(|rule| rule.apply("read_value")),
                Some(String::from("readValue")),
            );
        }
    }

    #[test]
    fn reads_container_tag_and_content() {
        let container =
            SerdeContainer::new(&[parse_quote!(#[serde(tag = "type", content = "data")])]).unwrap();

        assert_eq!(container.tag(), Some(String::from("type")));
        assert_eq!(container.content(), Some(String::from("data")));
    }

    #[test]
    fn reads_container_untagged_and_transparent() {
        let container = SerdeContainer::new(&[
            parse_quote!(#[serde(untagged)]),
            parse_quote!(#[serde(transparent)]),
        ])
        .unwrap();

        assert!(container.is_untagged());
        assert!(container.is_transparent());
    }

    #[test]
    fn reads_container_rename_all_fields() {
        assert_eq!(
            SerdeContainer::new(&[parse_quote!(#[serde(rename_all_fields = "camelCase")])])
                .unwrap()
                .rename_all_fields()
                .map(|rule| rule.apply("read_value")),
            Some(String::from("readValue")),
        );
    }

    #[test]
    fn reads_variant_rename_and_repeated_aliases() {
        assert_eq!(
            SerdeVariant::new(&[
                parse_quote!(#[serde(rename = "first", alias = "second", alias = "third")]),
            ])
            .unwrap()
            .names(&parse_quote!(Variant), None),
            ["first", "second", "third"],
        );
    }

    #[test]
    fn reads_variant_rename_all() {
        assert_eq!(
            SerdeVariant::new(&[parse_quote!(#[serde(rename_all = "camelCase")])])
                .unwrap()
                .rename_all()
                .map(|rule| rule.apply("read_value")),
            Some(String::from("readValue")),
        );
    }

    #[test]
    fn reads_variant_skip_skip_deserializing_and_other() {
        for skip in [quote!(skip), quote!(skip_deserializing), quote!(other)] {
            assert!(
                SerdeVariant::new(&[parse_quote!(#[serde(#skip)])])
                    .unwrap()
                    .is_skipped()
            );
        }
    }

    #[test]
    fn reads_variant_untagged() {
        assert!(
            SerdeVariant::new(&[parse_quote!(#[serde(untagged)])])
                .unwrap()
                .is_untagged()
        );
    }
}
