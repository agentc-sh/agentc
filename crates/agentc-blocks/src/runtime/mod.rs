// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use flate2::read::GzDecoder;
use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
};
use tar::Archive;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("I/O error extracting asset '{name}': {source}")]
    Io {
        name: &'static str,
        #[source]
        source: io::Error,
    },

    #[error("task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
}

/// How an [`EmbeddedAsset`] should be materialized on disk.
pub enum ExtractionMode {
    /// A gzipped tar archive. The top-level versioned directory component is
    /// stripped during extraction, mirroring the layout produced by `cargo package`.
    TarGz,
    /// A raw file written as-is to `{extraction_dir}/{name}`.
    Raw,
}

/// A binary asset embedded directly into the compiler binary.
pub struct EmbeddedAsset {
    pub name: &'static str,
    pub bytes: &'static [u8],
    pub mode: ExtractionMode,
}

impl EmbeddedAsset {
    /// Extract this asset into `into`, returning the path of the extracted root.
    ///
    /// For [`ExtractionMode::TarGz`] the archive is unpacked into `into/{name}/`
    /// with the top-level versioned path component stripped. For
    /// [`ExtractionMode::Raw`] the bytes are written directly to `into/{name}`.
    pub async fn extract(&'static self, into: PathBuf) -> Result<PathBuf, RuntimeError> {
        tokio::task::spawn_blocking(move || self.extract_sync(&into)).await?
    }

    fn extract_sync(&'static self, into: &Path) -> Result<PathBuf, RuntimeError> {
        let dest = into.join(self.name);

        if dest.exists() {
            std::fs::remove_dir_all(&dest)
                .map_err(|e| RuntimeError::Io { name: self.name, source: e })?;
        }

        std::fs::create_dir_all(&dest)
            .map_err(|e| RuntimeError::Io { name: self.name, source: e })?;

        match self.mode {
            ExtractionMode::TarGz => {
                let gz = GzDecoder::new(self.bytes);
                let mut archive = Archive::new(gz);

                for entry in archive
                    .entries()
                    .map_err(|e| RuntimeError::Io { name: self.name, source: e })?
                {
                    let mut entry =
                        entry.map_err(|e| RuntimeError::Io { name: self.name, source: e })?;
                    let entry_path = entry
                        .path()
                        .map_err(|e| RuntimeError::Io { name: self.name, source: e })?;

                    let target = dest.join(entry_path);

                    if let Some(parent) = target.parent() {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| RuntimeError::Io { name: self.name, source: e })?;
                    }

                    entry
                        .unpack(&target)
                        .map_err(|e| RuntimeError::Io { name: self.name, source: e })?;
                }
            }
            ExtractionMode::Raw => {
                std::fs::write(dest.join(self.name), self.bytes)
                    .map_err(|e| RuntimeError::Io { name: self.name, source: e })?;
            }
        }

        Ok(dest)
    }
}

/// Extraction behavior for a collection of embedded assets.
#[async_trait]
pub trait ExtractAll {
    /// Extract every asset into `into`, returning a map of asset name to extracted path.
    async fn extract_all(&self, into: PathBuf) -> Result<HashMap<String, PathBuf>, RuntimeError>;
}

#[async_trait]
impl ExtractAll for [&'static EmbeddedAsset] {
    async fn extract_all(&self, into: PathBuf) -> Result<HashMap<String, PathBuf>, RuntimeError> {
        let mut map = HashMap::with_capacity(self.len());

        for asset in self {
            let path = asset.extract(into.clone()).await?;

            map.insert(asset.name.to_string(), path);
        }

        Ok(map)
    }
}

/// All runtime crates embedded into the compiler binary.
///
/// This list must stay in sync with `RUNTIME_CRATES` in `build.rs` and the
/// dependencies in `Cargo.toml.j2`. When adding a new runtime crate, update
/// all three locations.
pub static EMBEDDED_RUNTIME: &[EmbeddedAsset] = &[
    EmbeddedAsset {
        name: "agentc-agent",
        bytes: include_bytes!("../../embedded/agentc-agent.crate"),
        mode: ExtractionMode::TarGz,
    },
    EmbeddedAsset {
        name: "agentc-agent-macros",
        bytes: include_bytes!("../../embedded/agentc-agent-macros.crate"),
        mode: ExtractionMode::TarGz,
    },
    EmbeddedAsset {
        name: "agentc-agent-react",
        bytes: include_bytes!("../../embedded/agentc-agent-react.crate"),
        mode: ExtractionMode::TarGz,
    },
    EmbeddedAsset {
        name: "agentc-config",
        bytes: include_bytes!("../../embedded/agentc-config.crate"),
        mode: ExtractionMode::TarGz,
    },
    EmbeddedAsset {
        name: "agentc-database",
        bytes: include_bytes!("../../embedded/agentc-database.crate"),
        mode: ExtractionMode::TarGz,
    },
    EmbeddedAsset {
        name: "agentc-domain",
        bytes: include_bytes!("../../embedded/agentc-domain.crate"),
        mode: ExtractionMode::TarGz,
    },
    EmbeddedAsset {
        name: "agentc-domain-sql",
        bytes: include_bytes!("../../embedded/agentc-domain-sql.crate"),
        mode: ExtractionMode::TarGz,
    },
    EmbeddedAsset {
        name: "agentc-http",
        bytes: include_bytes!("../../embedded/agentc-http.crate"),
        mode: ExtractionMode::TarGz,
    },
    EmbeddedAsset {
        name: "agentc-mcp",
        bytes: include_bytes!("../../embedded/agentc-mcp.crate"),
        mode: ExtractionMode::TarGz,
    },
    EmbeddedAsset {
        name: "agentc-model",
        bytes: include_bytes!("../../embedded/agentc-model.crate"),
        mode: ExtractionMode::TarGz,
    },
    EmbeddedAsset {
        name: "agentc-prompt",
        bytes: include_bytes!("../../embedded/agentc-prompt.crate"),
        mode: ExtractionMode::TarGz,
    },
    EmbeddedAsset {
        name: "agentc-protocol-ag-ui",
        bytes: include_bytes!("../../embedded/agentc-protocol-ag-ui.crate"),
        mode: ExtractionMode::TarGz,
    },
    EmbeddedAsset {
        name: "agentc-protocol-a2a",
        bytes: include_bytes!("../../embedded/agentc-protocol-a2a.crate"),
        mode: ExtractionMode::TarGz,
    },
    EmbeddedAsset {
        name: "agentc-skills",
        bytes: include_bytes!("../../embedded/agentc-skills.crate"),
        mode: ExtractionMode::TarGz,
    },
    EmbeddedAsset {
        name: "agentc-telemetry",
        bytes: include_bytes!("../../embedded/agentc-telemetry.crate"),
        mode: ExtractionMode::TarGz,
    },
    EmbeddedAsset {
        name: "agentc-tools",
        bytes: include_bytes!("../../embedded/agentc-tools.crate"),
        mode: ExtractionMode::TarGz,
    },
];
