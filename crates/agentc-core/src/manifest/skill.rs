// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use sanitizer::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::{Validate, ValidateArgs, ValidationErrors};

use agentc_compiler::asset::types::{AssetOrigin, AssetRef};

/// A skill definition as declared in the manifest.
///
/// Skills may be declared in two ways: as an embedded directory containing a
/// `SKILL.md` and optional supporting files (`source`), or with the skill body
/// and optional resources inlined directly into the manifest (`content`). Both
/// variants are baked into the compiled binary with no external dependencies at
/// runtime.
#[derive(Debug, Clone, Serialize, Deserialize, Sanitizer)]
#[serde(untagged)]
pub enum ManifestSkill {
    /// A skill loaded from a directory on disk. The directory must contain a
    /// `SKILL.md` file and may include any number of supporting resource files.
    Source(ManifestSkillSource),
    /// A skill whose body and optional resources are inlined directly in the manifest.
    Content(ManifestSkillContent),
}

impl ManifestSkill {
    /// Push an [`AssetRef`] for this skill into the given vector.
    ///
    /// Only [`ManifestSkill::Source`] variants produce an asset reference;
    /// inlined skills carry their content directly and require no asset fetch.
    pub fn collect_assets(&self, name: &str, assets: &mut Vec<AssetRef>) {
        if let ManifestSkill::Source(s) = self {
            assets.push(AssetRef::new(s.source.clone(), AssetOrigin::skill(name)));
        }
    }
}

impl<'v_a> ValidateArgs<'v_a> for ManifestSkill {
    type Args = ();

    fn validate_with_args(&self, args: Self::Args) -> Result<(), ValidationErrors> {
        match self {
            ManifestSkill::Source(s) => s.validate_with_args(args),
            ManifestSkill::Content(c) => c.validate_with_args(args),
        }
    }
}

impl Validate for ManifestSkill {
    fn validate(&self) -> Result<(), ValidationErrors> {
        self.validate_with_args(())
    }
}

/// Fields for a skill loaded from a directory.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestSkillSource {
    /// Path to the skill directory (containing `SKILL.md`), relative to the manifest file.
    #[validate(length(min = 1))]
    #[sanitizer(trim)]
    pub source: String,
}

/// Fields for a skill whose body (and optional resources) are inlined in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct ManifestSkillContent {
    /// Human-readable description of the skill's purpose.
    #[validate(length(min = 1))]
    #[sanitizer(trim)]
    pub description: String,

    /// The full markdown body of the skill (without frontmatter).
    #[validate(length(min = 1))]
    #[sanitizer(trim)]
    pub content: String,

    /// Optional supporting resource files for this skill, keyed by relative path.
    ///
    /// Each entry maps a relative path (e.g. `"scripts/run.sh"`) to the file's
    /// full text content, letting users inline scripts and reference files
    /// alongside the skill body without creating a directory on disk.
    #[serde(default)]
    pub resources: HashMap<String, String>,
}
