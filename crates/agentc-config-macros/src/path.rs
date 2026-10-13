// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use syn::{Ident, Path, parse_quote};

pub(crate) struct CratePath {
    explicit: Option<Path>,
}

impl CratePath {
    pub(crate) fn new(explicit: Option<Path>) -> Self {
        Self { explicit }
    }

    fn resolved() -> Path {
        match crate_name("agentc-config") {
            Ok(FoundCrate::Itself) => parse_quote!(::agentc_config),
            Ok(FoundCrate::Name(name)) => {
                let ident = Ident::new(&name.replace('-', "_"), Span::call_site());

                parse_quote!(::#ident)
            }
            Err(_) => parse_quote!(::agentc_config),
        }
    }

    pub(crate) fn resolve(self) -> Path {
        self.explicit
            .unwrap_or_else(Self::resolved)
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::parse_quote;

    use super::CratePath;

    #[test]
    fn resolves_to_agentc_config_by_default() {
        assert_eq!(
            CratePath::new(None)
                .resolve()
                .into_token_stream()
                .to_string(),
            ":: agentc_config",
        );
    }

    #[test]
    fn preserves_an_explicit_path() {
        assert_eq!(
            CratePath::new(Some(parse_quote!(custom::agentc_config)))
                .resolve()
                .into_token_stream()
                .to_string(),
            "custom :: agentc_config",
        );
    }
}
