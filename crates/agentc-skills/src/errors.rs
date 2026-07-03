// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("skill '{name}' has no description and cannot be loaded")]
    MissingDescription { name: String },

    #[error("skill frontmatter could not be parsed: {0}")]
    UnparsableFrontmatter(String),

    #[error("skill directory '{path}' could not be read: {source}")]
    IoError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("resource '{path}' not found in skill '{name}'")]
    ResourceNotFound { name: String, path: String },
}

impl SkillError {
    pub fn missing_description(name: impl Into<String>) -> Self {
        Self::MissingDescription { name: name.into() }
    }

    pub fn unparsable_frontmatter(msg: impl Into<String>) -> Self {
        Self::UnparsableFrontmatter(msg.into())
    }

    pub fn io_error(path: impl Into<String>, source: std::io::Error) -> Self {
        Self::IoError { path: path.into(), source }
    }

    pub fn resource_not_found(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self::ResourceNotFound { name: name.into(), path: path.into() }
    }
}
