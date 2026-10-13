// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use darling::{
    FromDeriveInput, FromField, FromVariant,
    ast::{Data, Fields, Style},
    util::{Flag, parse_expr::preserve_str_literal},
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, DeriveInput, Expr, Generics, Ident, Path, Type, WherePredicate, parse_quote};

use crate::{
    attributes::{SerdeContainer, SerdeField, SerdeVariant},
    naming::{Naming, RenameRule},
    path::CratePath,
};

enum Declaration {
    Default(Expr),
    Constant(Expr),
    Seal,
    Nested(Type),
}

enum MemberPath {
    Key(String),
    Index(usize),
    Same,
}

struct Member {
    path: MemberPath,
    declaration: Declaration,
}

impl Member {
    fn from_fields(
        fields: Fields<ConfigSchemaField>,
        rename_all: Option<RenameRule>,
        transparent: bool,
    ) -> Result<Vec<Self>, darling::Error> {
        let mut errors = darling::Error::accumulator();
        let same = transparent || (fields.style == Style::Tuple && fields.len() == 1);
        let members = fields
            .into_iter()
            .enumerate()
            .filter_map(|(index, field)| {
                errors
                    .handle(field.into_member(index, same, rename_all))
                    .flatten()
            })
            .collect::<Vec<_>>();

        errors.finish_with(members)
    }

    fn tokens(&self, crate_path: &Path) -> TokenStream {
        let path = match &self.path {
            MemberPath::Key(key) => quote! { path.child(#crate_path::path::Segment::key(#key)) },
            MemberPath::Index(index) => {
                quote! { path.child(#crate_path::path::Segment::index(#index)) }
            }
            MemberPath::Same => quote! { path.clone() },
        };

        match &self.declaration {
            Declaration::Default(value) => quote! {
                let declarations = declarations.default(#path, #value);
            },
            Declaration::Constant(value) => quote! {
                let declarations = declarations.constant(#path, #value);
            },
            Declaration::Seal => quote! {
                let declarations = declarations.seal(#path);
            },
            Declaration::Nested(ty) => quote! {
                let declarations = <#ty as #crate_path::traits::ConfigSchema>::declare(
                    declarations,
                    &#path,
                );
            },
        }
    }

    fn predicate(&self, crate_path: &Path) -> Option<WherePredicate> {
        match &self.declaration {
            Declaration::Nested(ty) => Some(parse_quote!(#ty: #crate_path::traits::ConfigSchema)),
            _ => None,
        }
    }
}

#[derive(FromField)]
#[darling(attributes(config), forward_attrs(serde))]
struct ConfigSchemaField {
    ident: Option<Ident>,
    ty: Type,
    attrs: Vec<Attribute>,
    #[darling(with = preserve_str_literal, map = Some)]
    default: Option<Expr>,
    #[darling(with = preserve_str_literal, map = Some)]
    constant: Option<Expr>,
    seal: Flag,
    nested: Flag,
}

impl ConfigSchemaField {
    fn into_member(
        self,
        index: usize,
        same: bool,
        rename_all: Option<RenameRule>,
    ) -> Result<Option<Member>, darling::Error> {
        let serde = SerdeField::new(&self.attrs)?;
        let declaration =
            match (self.default, self.constant, self.seal.is_present(), self.nested.is_present()) {
                (None, None, false, false) => return Ok(None),
                (Some(value), None, false, false) => Declaration::Default(value),
                (None, Some(value), false, false) => Declaration::Constant(value),
                (None, None, true, false) => Declaration::Seal,
                (None, None, false, true) => Declaration::Nested(self.ty.clone()),
                _ => {
                    return Err(darling::Error::custom(
                        "a field takes one of `default`, `constant`, `seal` or `nested`",
                    )
                    .with_span(&self.ty));
                }
            };

        if serde.is_skipped() {
            return Err(darling::Error::custom("a skipped field has no config declarations")
                .with_span(&self.ty));
        }

        match (serde.is_flatten(), declaration, same, self.ident) {
            (true, declaration @ Declaration::Nested(_), ..) => {
                Ok(Some(Member { path: MemberPath::Same, declaration }))
            }
            (true, ..) => {
                Err(darling::Error::custom("a flattened field takes only `nested`")
                    .with_span(&self.ty))
            }
            (false, declaration, true, _) => {
                Ok(Some(Member { path: MemberPath::Same, declaration }))
            }
            (false, declaration, false, Some(ident)) => Ok(Some(Member {
                path: MemberPath::Key(Naming::member(&ident, serde.rename(), rename_all)),
                declaration,
            })),
            (false, declaration, false, None) => Ok(Some(Member {
                path: MemberPath::Index(index),
                declaration,
            })),
        }
    }
}

enum Representation {
    External,
    Internal { tag: String },
    Adjacent { tag: String, content: String },
    Untagged,
}

impl Representation {
    fn new(container: &SerdeContainer) -> Self {
        match (container.is_untagged(), container.tag(), container.content()) {
            (true, ..) => Self::Untagged,
            (false, Some(tag), Some(content)) => Self::Adjacent { tag, content },
            (false, Some(tag), None) => Self::Internal { tag },
            (false, None, _) => Self::External,
        }
    }

    fn scope(&self, crate_path: &Path, name: &str) -> Option<(TokenStream, TokenStream)> {
        match self {
            Self::External => Some((
                quote! { #crate_path::declarations::VariantTag::Key },
                quote! { let path = path.child(#crate_path::path::Segment::key(#name)); },
            )),
            Self::Internal { tag } => Some((
                quote! { #crate_path::declarations::VariantTag::Field(#tag.into()) },
                TokenStream::new(),
            )),
            Self::Adjacent { tag, content } => Some((
                quote! { #crate_path::declarations::VariantTag::Field(#tag.into()) },
                quote! { let path = path.child(#crate_path::path::Segment::key(#content)); },
            )),
            Self::Untagged => None,
        }
    }
}

struct Variant {
    names: Vec<String>,
    members: Vec<Member>,
}

impl Variant {
    fn members(&self) -> &[Member] {
        &self.members
    }

    fn tokens(&self, representation: &Representation, crate_path: &Path) -> TokenStream {
        let members = self
            .members
            .iter()
            .map(|member| member.tokens(crate_path))
            .collect::<Vec<_>>();

        self.names
            .iter()
            .filter(|_| !members.is_empty())
            .filter_map(|name| {
                representation
                    .scope(crate_path, name)
                    .map(|(tag, base)| (name, tag, base))
            })
            .map(|(name, tag, base)| {
                quote! {
                    let declarations = declarations.variant(
                        #crate_path::declarations::VariantCondition::new(path.clone(), #tag, #name),
                        |declarations| {
                            #base
                            #(#members)*

                            declarations
                        },
                    );
                }
            })
            .collect()
    }
}

#[derive(FromVariant)]
#[darling(attributes(config), forward_attrs(serde))]
struct ConfigSchemaVariant {
    ident: Ident,
    fields: Fields<ConfigSchemaField>,
    attrs: Vec<Attribute>,
}

impl ConfigSchemaVariant {
    fn into_variant(
        self,
        container: &SerdeContainer,
        representation: &Representation,
    ) -> Result<Variant, darling::Error> {
        let serde = SerdeVariant::new(&self.attrs)?;
        let names = serde.names(&self.ident, container.rename_all());
        let members = Member::from_fields(
            self.fields,
            serde
                .rename_all()
                .or(container.rename_all_fields()),
            false,
        )?;

        match (
            members.is_empty(),
            serde.is_skipped(),
            serde.is_untagged() || matches!(representation, Representation::Untagged),
        ) {
            (true, ..) => Ok(Variant { names, members }),
            (false, true, _) => {
                Err(darling::Error::custom("a skipped variant has no config declarations")
                    .with_span(&self.ident))
            }
            (false, false, true) => {
                Err(darling::Error::custom("an untagged variant has no config declarations")
                    .with_span(&self.ident))
            }
            (false, false, false) => Ok(Variant { names, members }),
        }
    }
}

enum Shape {
    Struct(Vec<Member>),
    Enum(Representation, Vec<Variant>),
}

impl Shape {
    fn members(&self) -> Vec<&Member> {
        match self {
            Self::Struct(members) => members.iter().collect(),
            Self::Enum(_, variants) => variants
                .iter()
                .flat_map(Variant::members)
                .collect(),
        }
    }

    fn tokens(&self, crate_path: &Path) -> TokenStream {
        match self {
            Self::Struct(members) => members
                .iter()
                .map(|member| member.tokens(crate_path))
                .collect(),
            Self::Enum(representation, variants) => variants
                .iter()
                .map(|variant| variant.tokens(representation, crate_path))
                .collect(),
        }
    }
}

#[derive(FromDeriveInput)]
#[darling(
    attributes(config),
    supports(struct_any, enum_any),
    forward_attrs(serde)
)]
struct ConfigSchemaInput {
    ident: Ident,
    generics: Generics,
    data: Data<ConfigSchemaVariant, ConfigSchemaField>,
    attrs: Vec<Attribute>,
    crate_path: Option<Path>,
    #[darling(with = preserve_str_literal, map = Some)]
    default: Option<Expr>,
}

impl ConfigSchemaInput {
    fn into_derive(self) -> Result<ConfigSchemaDerive, darling::Error> {
        let container = SerdeContainer::new(&self.attrs)?;
        let shape = match self.data {
            Data::Struct(fields) => Shape::Struct(Member::from_fields(
                fields,
                container.rename_all(),
                container.is_transparent(),
            )?),
            Data::Enum(variants) => {
                let representation = Representation::new(&container);
                let mut errors = darling::Error::accumulator();
                let variants = variants
                    .into_iter()
                    .filter_map(|variant| {
                        errors.handle(variant.into_variant(&container, &representation))
                    })
                    .collect::<Vec<_>>();

                Shape::Enum(representation, errors.finish_with(variants)?)
            }
        };

        Ok(ConfigSchemaDerive {
            ident: self.ident,
            generics: self.generics,
            crate_path: CratePath::new(self.crate_path).resolve(),
            default: self.default,
            shape,
        })
    }
}

pub(crate) struct ConfigSchemaDerive {
    ident: Ident,
    generics: Generics,
    crate_path: Path,
    default: Option<Expr>,
    shape: Shape,
}

impl ConfigSchemaDerive {
    pub(crate) fn new(input: &DeriveInput) -> Result<Self, darling::Error> {
        ConfigSchemaInput::from_derive_input(input)?.into_derive()
    }

    pub(crate) fn expand(&self) -> TokenStream {
        let crate_path = &self.crate_path;
        let ident = &self.ident;
        let mut implementation = self.generics.clone();

        implementation
            .make_where_clause()
            .predicates
            .extend(
                self.shape
                    .members()
                    .into_iter()
                    .filter_map(|member| member.predicate(crate_path)),
            );

        let (impl_generics, _, where_clause) = implementation.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();
        let default = self
            .default
            .as_ref()
            .map(|value| quote! { let declarations = declarations.default(path.clone(), #value); });
        let declarations = self.shape.tokens(crate_path);

        quote! {
            impl #impl_generics #crate_path::traits::ConfigSchema
                for #ident #ty_generics #where_clause
            {
                #[allow(unused_variables)]
                fn declare(
                    declarations: #crate_path::declarations::Declarations,
                    path: &#crate_path::path::Path,
                ) -> #crate_path::declarations::Declarations {
                    #default
                    #declarations

                    declarations
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::{ToTokens, quote};
    use syn::{DeriveInput, ItemImpl, parse_quote, parse2};

    use super::ConfigSchemaDerive;

    struct Expansion(Result<String, String>);

    impl Expansion {
        fn new(input: DeriveInput) -> Self {
            Self(
                ConfigSchemaDerive::new(&input)
                    .map(|derive| {
                        parse2::<ItemImpl>(derive.expand())
                            .unwrap()
                            .to_token_stream()
                            .to_string()
                    })
                    .map_err(|error| error.to_string()),
            )
        }

        fn count(&self, tokens: TokenStream) -> usize {
            self.0
                .as_ref()
                .unwrap()
                .matches(&tokens.to_string())
                .count()
        }

        fn error(&self) -> &str {
            self.0.as_ref().unwrap_err()
        }
    }

    #[test]
    fn emits_default_at_the_field_key() {
        assert_eq!(
            Expansion::new(parse_quote! {
                struct Config {
                    #[config(default = 8080)]
                    port: u16,
                }
            })
            .count(quote! {
                let declarations = declarations.default(
                    path.child(::agentc_config::path::Segment::key("port")),
                    8080
                );
            }),
            1,
        );
    }

    #[test]
    fn emits_constant_at_the_field_key() {
        assert_eq!(
            Expansion::new(parse_quote! {
                struct Config {
                    #[config(constant = 8080)]
                    port: u16,
                }
            })
            .count(quote! {
                let declarations = declarations.constant(
                    path.child(::agentc_config::path::Segment::key("port")),
                    8080
                );
            }),
            1,
        );
    }

    #[test]
    fn keeps_a_string_literal_value_as_written() {
        assert_eq!(
            Expansion::new(parse_quote! {
                struct Config {
                    #[config(default = "redis://localhost:6379")]
                    url: String,
                }
            })
            .count(quote! {
                let declarations = declarations.default(
                    path.child(::agentc_config::path::Segment::key("url")),
                    "redis://localhost:6379"
                );
            }),
            1,
        );
    }

    #[test]
    fn emits_seal_at_the_field_key() {
        assert_eq!(
            Expansion::new(parse_quote! {
                struct Config {
                    #[config(seal)]
                    port: u16,
                }
            })
            .count(quote! {
                let declarations = declarations.seal(
                    path.child(::agentc_config::path::Segment::key("port"))
                );
            }),
            1,
        );
    }

    #[test]
    fn emits_nested_declare_and_where_predicate() {
        let expansion = Expansion::new(parse_quote! {
            struct Config<T> {
                #[config(nested)]
                inner: T,
            }
        });

        assert_eq!(
            expansion.count(quote! {
                let declarations = <T as ::agentc_config::traits::ConfigSchema>::declare(
                    declarations,
                    &path.child(::agentc_config::path::Segment::key("inner")),
                );
            }),
            1,
        );
        assert_eq!(expansion.count(quote! { where T: ::agentc_config::traits::ConfigSchema }), 1,);
    }

    #[test]
    fn unattributed_field_emits_nothing() {
        assert_eq!(
            Expansion::new(parse_quote! {
                struct Config {
                    port: u16,
                }
            })
            .count(quote! { let declarations }),
            0,
        );
    }

    #[test]
    fn rename_all_and_rename_change_the_key() {
        let expansion = Expansion::new(parse_quote! {
            #[serde(rename_all = "camelCase")]
            struct Config {
                #[config(seal)]
                listen_port: u16,
                #[serde(rename = "who")]
                #[config(seal)]
                identity: u8,
            }
        });

        assert_eq!(
            expansion.count(quote! { ::agentc_config::path::Segment::key("listenPort") }),
            1,
        );
        assert_eq!(expansion.count(quote! { ::agentc_config::path::Segment::key("who") }), 1,);
    }

    #[test]
    fn flattened_nested_field_declares_at_the_same_path() {
        assert_eq!(
            Expansion::new(parse_quote! {
                struct Config {
                    #[serde(flatten)]
                    #[config(nested)]
                    inner: Inner,
                }
            })
            .count(quote! {
                let declarations = <Inner as ::agentc_config::traits::ConfigSchema>::declare(
                    declarations,
                    &path.clone(),
                );
            }),
            1,
        );
    }

    #[test]
    fn crate_path_replaces_agentc_config() {
        let expansion = Expansion::new(parse_quote! {
            #[config(crate_path = custom::config)]
            struct Config {
                #[config(seal)]
                port: u16,
            }
        });

        assert_eq!(expansion.count(quote! { ::agentc_config }), 0);
        assert_eq!(
            expansion.count(quote! { impl custom::config::traits::ConfigSchema for Config }),
            1,
        );
    }

    #[test]
    fn external_variant_conditions_on_its_key() {
        let expansion = Expansion::new(parse_quote! {
            enum PubSub {
                Redis {
                    #[config(seal)]
                    url: String,
                },
            }
        });

        assert_eq!(
            expansion.count(quote! {
                ::agentc_config::declarations::VariantCondition::new(
                    path.clone(),
                    ::agentc_config::declarations::VariantTag::Key,
                    "Redis"
                )
            }),
            1,
        );
        assert_eq!(
            expansion.count(quote! {
                let path = path.child(::agentc_config::path::Segment::key("Redis"));
            }),
            1,
        );
    }

    #[test]
    fn internal_variant_conditions_on_its_tag() {
        let expansion = Expansion::new(parse_quote! {
            #[serde(tag = "kind")]
            enum PubSub {
                Redis {
                    #[config(seal)]
                    url: String,
                },
            }
        });

        assert_eq!(
            expansion.count(quote! {
                ::agentc_config::declarations::VariantCondition::new(
                    path.clone(),
                    ::agentc_config::declarations::VariantTag::Field("kind".into()),
                    "Redis"
                )
            }),
            1,
        );
        assert_eq!(expansion.count(quote! { let path }), 0);
    }

    #[test]
    fn adjacent_variant_conditions_on_its_tag_and_binds_its_content() {
        let expansion = Expansion::new(parse_quote! {
            #[serde(tag = "kind", content = "data")]
            enum PubSub {
                Redis {
                    #[config(seal)]
                    url: String,
                },
            }
        });

        assert_eq!(
            expansion.count(quote! {
                ::agentc_config::declarations::VariantCondition::new(
                    path.clone(),
                    ::agentc_config::declarations::VariantTag::Field("kind".into()),
                    "Redis"
                )
            }),
            1,
        );
        assert_eq!(
            expansion.count(quote! {
                let path = path.child(::agentc_config::path::Segment::key("data"));
            }),
            1,
        );
    }

    #[test]
    fn emits_one_variant_per_alias() {
        let expansion = Expansion::new(parse_quote! {
            #[serde(tag = "kind")]
            enum PubSub {
                #[serde(alias = "valkey")]
                Redis {
                    #[config(seal)]
                    url: String,
                },
            }
        });

        assert_eq!(expansion.count(quote! { declarations.variant }), 2);
        assert_eq!(expansion.count(quote! { "Redis" }), 1);
        assert_eq!(expansion.count(quote! { "valkey" }), 1);
    }

    #[test]
    fn unit_variant_emits_nothing() {
        assert_eq!(
            Expansion::new(parse_quote! {
                enum PubSub {
                    Disabled,
                }
            })
            .count(quote! { let declarations }),
            0,
        );
    }

    #[test]
    fn newtype_and_tuple_variant_paths() {
        let expansion = Expansion::new(parse_quote! {
            enum Target {
                One(#[config(seal)] u8),
                Two(#[config(seal)] u8, #[config(seal)] u8),
            }
        });

        assert_eq!(
            expansion.count(quote! { let declarations = declarations.seal(path.clone()); }),
            1,
        );
        assert_eq!(expansion.count(quote! { ::agentc_config::path::Segment::index(0usize) }), 1,);
        assert_eq!(expansion.count(quote! { ::agentc_config::path::Segment::index(1usize) }), 1,);
    }

    #[test]
    fn newtype_struct_declares_at_its_own_path() {
        assert_eq!(
            Expansion::new(parse_quote! {
                struct Port(#[config(seal)] u16);
            })
            .count(quote! { let declarations = declarations.seal(path.clone()); }),
            1,
        );
    }

    #[test]
    fn tuple_struct_declares_at_each_index() {
        let expansion = Expansion::new(parse_quote! {
            struct Pair(#[config(seal)] u8, #[config(seal)] u8);
        });

        assert_eq!(expansion.count(quote! { ::agentc_config::path::Segment::index(0usize) }), 1,);
        assert_eq!(expansion.count(quote! { ::agentc_config::path::Segment::index(1usize) }), 1,);
    }

    #[test]
    fn transparent_struct_declares_at_its_own_path() {
        assert_eq!(
            Expansion::new(parse_quote! {
                #[serde(transparent)]
                struct Wrapper {
                    #[config(seal)]
                    inner: u8,
                }
            })
            .count(quote! { let declarations = declarations.seal(path.clone()); }),
            1,
        );
    }

    #[test]
    fn container_default_declares_before_the_members() {
        assert_eq!(
            Expansion::new(parse_quote! {
                #[config(default = 1)]
                struct Config {
                    #[config(seal)]
                    port: u16,
                }
            })
            .count(quote! {
                let declarations = declarations.default(path.clone(), 1);
                let declarations = declarations.seal(
                    path.child(::agentc_config::path::Segment::key("port"))
                );
            }),
            1,
        );
    }

    #[test]
    fn rename_all_renames_variants_and_rename_all_fields_renames_their_fields() {
        let expansion = Expansion::new(parse_quote! {
            #[serde(tag = "kind", rename_all = "snake_case", rename_all_fields = "camelCase")]
            enum PubSub {
                RedisCache {
                    #[config(seal)]
                    pool_size: u32,
                },
            }
        });

        assert_eq!(expansion.count(quote! { "redis_cache" }), 1);
        assert_eq!(expansion.count(quote! { ::agentc_config::path::Segment::key("poolSize") }), 1,);
    }

    #[test]
    fn nested_where_predicates_from_variant_fields() {
        assert_eq!(
            Expansion::new(parse_quote! {
                enum Target<T> {
                    One {
                        #[config(nested)]
                        inner: T,
                    },
                }
            })
            .count(quote! { where T: ::agentc_config::traits::ConfigSchema }),
            1,
        );
    }

    #[test]
    fn rejects_two_declarations_on_one_field() {
        assert_eq!(
            Expansion::new(parse_quote! {
                struct Config {
                    #[config(default = 1, seal)]
                    port: u16,
                }
            })
            .error(),
            "a field takes one of `default`, `constant`, `seal` or `nested`",
        );
    }

    #[test]
    fn rejects_declarations_on_a_skipped_field() {
        for skip in [quote!(skip), quote!(skip_deserializing)] {
            assert_eq!(
                Expansion::new(parse_quote! {
                    struct Config {
                        #[serde(#skip)]
                        #[config(seal)]
                        port: u16,
                    }
                })
                .error(),
                "a skipped field has no config declarations",
            );
        }
    }

    #[test]
    fn rejects_other_declarations_on_a_flattened_field() {
        for declaration in [quote!(default = 1), quote!(constant = 1), quote!(seal)] {
            assert_eq!(
                Expansion::new(parse_quote! {
                    struct Config {
                        #[serde(flatten)]
                        #[config(#declaration)]
                        inner: Inner,
                    }
                })
                .error(),
                "a flattened field takes only `nested`",
            );
        }
    }

    #[test]
    fn rejects_an_unknown_rename_all_rule() {
        assert!(
            Expansion::new(parse_quote! {
                #[serde(rename_all = "Title Case")]
                struct Config {
                    port: u16,
                }
            })
            .error()
            .contains("Unknown value: `Title Case`")
        );
    }

    #[test]
    fn rejects_declarations_in_an_untagged_container_variant() {
        assert_eq!(
            Expansion::new(parse_quote! {
                #[serde(untagged)]
                enum Target {
                    One {
                        #[config(seal)]
                        port: u16,
                    },
                }
            })
            .error(),
            "an untagged variant has no config declarations",
        );
    }

    #[test]
    fn rejects_declarations_in_an_untagged_variant() {
        assert_eq!(
            Expansion::new(parse_quote! {
                enum Target {
                    #[serde(untagged)]
                    One {
                        #[config(seal)]
                        port: u16,
                    },
                }
            })
            .error(),
            "an untagged variant has no config declarations",
        );
    }

    #[test]
    fn rejects_declarations_in_a_skipped_variant() {
        for skip in [quote!(skip), quote!(skip_deserializing), quote!(other)] {
            assert_eq!(
                Expansion::new(parse_quote! {
                    enum Target {
                        #[serde(#skip)]
                        One {
                            #[config(seal)]
                            port: u16,
                        },
                    }
                })
                .error(),
                "a skipped variant has no config declarations",
            );
        }
    }

    #[test]
    fn rejects_config_on_a_variant() {
        assert!(
            Expansion::new(parse_quote! {
                enum Target {
                    #[config(seal)]
                    One {
                        port: u16,
                    },
                }
            })
            .error()
            .contains("Unknown field: `seal`")
        );
    }
}
