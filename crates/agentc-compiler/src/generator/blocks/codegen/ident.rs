// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

/// Converts an arbitrary string into a form usable as a Rust identifier.
///
/// Code generators derive identifiers from values that may contain characters
/// invalid in Rust identifiers, such as filesystem paths (`/`, `.`) or model IDs
/// (`llama3.1`). Feeding those directly to `proc_macro2::Ident::new` would panic,
/// so every non-alphanumeric character is replaced with `_`.
pub trait ToIdent {
    fn to_ident(&self) -> String;
}

impl ToIdent for str {
    fn to_ident(&self) -> String {
        self.chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_alphanumerics_untouched() {
        assert_eq!("plain123".to_ident(), "plain123");
    }

    #[test]
    fn replaces_every_non_alphanumeric_with_underscore() {
        assert_eq!("gpt-4o".to_ident(), "gpt_4o");
        assert_eq!("llama3.1".to_ident(), "llama3_1");
        assert_eq!("/abs/path/bundle.js".to_ident(), "_abs_path_bundle_js");
    }
}
