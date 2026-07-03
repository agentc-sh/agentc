// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod errors;
pub mod handler;
pub mod registry;
pub mod resolver;
pub mod store;
pub mod types;

pub use errors::AssetError;
pub use handler::{AssetHandler, LocalFileHandler};
pub use resolver::AssetResolver;
pub use store::ArtifactStore;
pub use types::{AssetOrigin, AssetRef, ResolvedAsset};
