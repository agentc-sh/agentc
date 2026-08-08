// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_compiler::generator::{errors::GeneratorError, extension::ExtensionPoint};

use crate::config::sections::contribution::{ConfigSectionSlot, ConfigSections};

#[derive(Debug, Clone)]
pub struct ConfigSectionsExtensionPoint {
    name: &'static str,
    slot: ConfigSectionSlot,
}

impl ConfigSectionsExtensionPoint {
    pub fn new(name: &'static str, slot: ConfigSectionSlot) -> Self {
        Self { name, slot }
    }
}

impl ExtensionPoint for ConfigSectionsExtensionPoint {
    type Contribution = ConfigSections;

    fn name(&self) -> &str {
        self.name
    }

    fn reduce(&self, contributions: Vec<Self::Contribution>) -> Result<String, GeneratorError> {
        Ok(ConfigSections::merge_all(contributions)
            .map_err(|error| GeneratorError::unexpected(error.to_string()))?
            .into_values()
            .map(|section| {
                section
                    .slot(self.slot)
                    .as_str()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use quote::quote;

    use crate::config::sections::contribution::ConfigSectionContribution;

    fn point(slot: ConfigSectionSlot) -> ConfigSectionsExtensionPoint {
        ConfigSectionsExtensionPoint::new("config::sections::test", slot)
    }

    fn sections(section: ConfigSectionContribution) -> ConfigSections {
        ConfigSections::from_entries([section]).unwrap()
    }

    #[test]
    fn sections_with_the_same_name_collapse_to_the_last_one() {
        assert_eq!(
            ExtensionPoint::reduce(
                &point(ConfigSectionSlot::Types),
                vec![
                    sections(
                        ConfigSectionContribution::new("a").types(quote! { pub struct First; }),
                    ),
                    sections(
                        ConfigSectionContribution::new("a").types(quote! { pub struct Second; }),
                    ),
                ],
            )
            .unwrap(),
            "pub struct Second ;",
        );
    }

    #[test]
    fn sections_render_in_name_order() {
        assert_eq!(
            ExtensionPoint::reduce(
                &point(ConfigSectionSlot::Types),
                vec![
                    sections(ConfigSectionContribution::new("zzz").types(quote! { struct Z; })),
                    sections(ConfigSectionContribution::new("aaa").types(quote! { struct A; })),
                ],
            )
            .unwrap(),
            "struct A ;\nstruct Z ;",
        );
    }

    #[test]
    fn a_point_projects_only_its_own_slot() {
        assert_eq!(
            ExtensionPoint::reduce(
                &point(ConfigSectionSlot::Fields),
                vec![sections(
                    ConfigSectionContribution::new("a")
                        .uses(quote! { use std::io; })
                        .types(quote! { pub struct A; })
                        .fields(quote! { pub a: A, })
                        .loader(quote! { .default(path!["a"], 1) })
                        .mapper(quote! { .field(path!["a"], "A") }),
                )],
            )
            .unwrap(),
            "pub a : A ,",
        );
    }
}
