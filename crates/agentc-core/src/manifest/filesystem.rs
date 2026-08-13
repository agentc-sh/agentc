// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use validator::Validate;

use agentc_blocks::context::ResolvedContextFilesystemBackend;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(default)]
pub struct ManifestFilesystem {
    /// The mounts making up the agent's virtual filesystem.
    #[validate(nested)]
    pub mounts: Vec<ManifestFilesystemMount>,
}

impl Default for ManifestFilesystem {
    fn default() -> Self {
        Self {
            mounts: vec![ManifestFilesystemMount {
                path: "/".to_string(),
                backend: ManifestFilesystemBackend::Memory,
            }],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ManifestFilesystemMount {
    /// Where the backend is mounted in the virtual filesystem.
    pub path: String,
    /// What backs this mount.
    pub backend: ManifestFilesystemBackend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ManifestFilesystemBackend {
    Memory,
    Host {
        root: String,
        #[serde(default)]
        follow_symlinks: bool,
    },
    ReadOnly {
        inner: Box<ManifestFilesystemBackend>,
    },
    Overlay {
        upper: Box<ManifestFilesystemBackend>,
        lower: Box<ManifestFilesystemBackend>,
    },
}

impl ManifestFilesystemBackend {
    pub fn resolve(&self) -> ResolvedContextFilesystemBackend {
        match self {
            Self::Memory => ResolvedContextFilesystemBackend::Memory,
            Self::Host { root, follow_symlinks } => ResolvedContextFilesystemBackend::Host {
                root: root.clone(),
                follow_symlinks: *follow_symlinks,
            },
            Self::ReadOnly { inner } => ResolvedContextFilesystemBackend::ReadOnly {
                inner: Box::new(inner.resolve()),
            },
            Self::Overlay { upper, lower } => ResolvedContextFilesystemBackend::Overlay {
                upper: Box::new(upper.resolve()),
                lower: Box::new(lower.resolve()),
            },
        }
    }
}
