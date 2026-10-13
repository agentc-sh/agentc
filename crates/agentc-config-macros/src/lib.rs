// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[allow(unused_extern_crates)]
extern crate self as agentc_config_macros;

mod attributes;
mod derive;
mod naming;
mod path;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

use crate::derive::ConfigSchemaDerive;

#[proc_macro_derive(ConfigSchema, attributes(config))]
pub fn derive_config_schema(input: TokenStream) -> TokenStream {
    match ConfigSchemaDerive::new(&parse_macro_input!(input as DeriveInput)) {
        Ok(derive) => derive.expand(),
        Err(error) => error.write_errors(),
    }
    .into()
}
