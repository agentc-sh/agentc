// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

/// A reference to a asset that needs to be fetched, along with metadata
/// about what it is for.
#[derive(Debug, Clone)]
pub struct AssetRef {
    /// The URI of the asset to fetch.
    pub uri: String,
    /// The origin of the asset, for error attribution.
    pub origin: AssetOrigin,
}

impl AssetRef {
    pub fn new(uri: impl Into<String>, origin: AssetOrigin) -> Self {
        Self { uri: uri.into(), origin }
    }
}

/// Describes what a asset is for, used for attributing errors.
#[derive(Debug, Clone)]
pub enum AssetOrigin {
    /// A tool defined in the manifest.
    Tool { name: String },
    /// A block defined in the manifest.
    Block { name: String },
    /// A skill defined in the manifest.
    Skill { name: String },
    /// An asset produced internally by the compiler (e.g. embedded runtime crates).
    Internal,
}

impl AssetOrigin {
    pub fn tool(name: impl Into<String>) -> Self {
        Self::Tool { name: name.into() }
    }

    pub fn block(name: impl Into<String>) -> Self {
        Self::Block { name: name.into() }
    }

    pub fn skill(name: impl Into<String>) -> Self {
        Self::Skill { name: name.into() }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedAsset {
    /// The local file path where the source content can be found.
    pub local_path: PathBuf,
    /// The URI of the fetched asset.
    pub uri: String,
    /// The origin of this asset.
    pub origin: AssetOrigin,
}
