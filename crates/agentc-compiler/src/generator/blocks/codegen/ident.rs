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
