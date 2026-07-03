// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::path::Path;

use crate::{
    asset::types::AssetOrigin,
    transformer::{
        errors::TransformError,
        traits::{AssetTransformer, TransformSink},
        types::AssetArtifact,
    },
};

pub struct SkillTransformer;

impl Default for SkillTransformer {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillTransformer {
    pub fn new() -> Self {
        Self
    }

    /// Walk the skill directory and produce one artifact per file.
    ///
    /// `SKILL.md` gets kind `"skill_md"`; every other file gets kind
    /// `"resource"`. The transformer never needs to compile or bundle
    /// anything. The paths are passed directly to codegen for `include_str!`.
    async fn collect_artifacts(&self, dir: &Path) -> Result<Vec<AssetArtifact>, TransformError> {
        let mut artifacts = Vec::new();
        let skill_md = dir.join("SKILL.md");

        if !tokio::fs::try_exists(&skill_md)
            .await
            .unwrap_or(false)
        {
            return Err(TransformError::failed(
                dir.to_string_lossy(),
                "skill directory is missing SKILL.md",
            ));
        }

        artifacts.push(AssetArtifact::path("skill_md", skill_md.clone()));

        let mut stack = vec![dir.to_path_buf()];

        while let Some(current) = stack.pop() {
            let mut entries = tokio::fs::read_dir(&current)
                .await
                .map_err(|e| TransformError::io(current.to_string_lossy(), e))?;

            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|e| TransformError::io(current.to_string_lossy(), e))?
            {
                let path = entry.path();

                if path == skill_md {
                    continue;
                }

                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    artifacts.push(AssetArtifact::path("resource", path));
                }
            }
        }

        Ok(artifacts)
    }
}

#[async_trait]
impl AssetTransformer for SkillTransformer {
    async fn can_transform(&self, local_path: &Path, origin: &AssetOrigin) -> bool {
        matches!(origin, AssetOrigin::Skill { .. }) && local_path.is_dir()
    }

    async fn transform(
        &self,
        local_path: &Path,
        _origin: &AssetOrigin,
        _sink: &dyn TransformSink,
    ) -> Result<Vec<AssetArtifact>, TransformError> {
        self.collect_artifacts(local_path).await
    }
}
