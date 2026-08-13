// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolvedContextFilesystem {
    /// The mounts making up the agent's virtual filesystem.
    pub mounts: Vec<ResolvedContextFilesystemMount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextFilesystemMount {
    /// Where the backend is mounted in the virtual filesystem.
    pub path: String,
    /// What backs this mount.
    pub backend: ResolvedContextFilesystemBackend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolvedContextFilesystemBackend {
    Memory,
    Host {
        root: String,
        follow_symlinks: bool,
    },
    ReadOnly {
        inner: Box<ResolvedContextFilesystemBackend>,
    },
    Overlay {
        upper: Box<ResolvedContextFilesystemBackend>,
        lower: Box<ResolvedContextFilesystemBackend>,
    },
}

impl ResolvedContextFilesystemBackend {
    pub fn tokens(&self) -> TokenStream {
        match self {
            Self::Memory => quote! {
                MemoryFs::new()
            },
            Self::Host { root, follow_symlinks } => quote! {
                HostFs::builder()
                    .root(#root)
                    .follow_symlinks(#follow_symlinks)
                    .build()?
            },
            Self::ReadOnly { inner } => {
                let inner = inner.tokens();

                quote! { ReadOnlyFs::new(#inner) }
            }
            Self::Overlay { upper, lower } => {
                let upper = upper.tokens();
                let lower = lower.tokens();

                quote! { OverlayFs::new(#upper, #lower) }
            }
        }
    }
}
