// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Resolved skill context, carrying everything codegen needs to embed a skill
/// into the compiled binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextSkill {
    /// The skill name as declared in the manifest.
    pub name: String,
    /// Kind-specific resolved data.
    pub kind: ResolvedContextSkillKind,
}

/// Discriminates between the two ways a skill may be declared.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResolvedContextSkillKind {
    /// A skill loaded from a directory on disk and baked in via `include_str!`.
    Source(ResolvedContextSkillSource),
    /// A skill whose body and optional resources are inlined in the manifest.
    Content(ResolvedContextSkillContent),
}

/// Resolved data for a skill baked in from a directory.
///
/// All paths are absolute and suitable for direct use in `include_str!` calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextSkillSource {
    /// Absolute path to the `SKILL.md` artifact.
    pub skill_md_path: String,
    /// Resource files bundled with this skill.
    ///
    /// Each entry is `(relative_path, absolute_artifact_path)`, where
    /// `relative_path` is the path as it will be keyed in the skill registry
    /// (e.g. `"scripts/run.sh"`) and `absolute_artifact_path` is the path
    /// used in the `include_str!` call.
    pub resources: Vec<(String, String)>,
}

/// Resolved data for a fully inlined skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextSkillContent {
    /// Human-readable description of the skill's purpose.
    pub description: String,
    /// The full markdown body of the skill (without frontmatter).
    pub content: String,
    /// Inlined resource files, keyed by relative path.
    pub resources: HashMap<String, String>,
}
