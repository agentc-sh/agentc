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
    /// A skill loaded from a directory on disk and baked into the binary.
    Source(ResolvedContextSkillSource),
    /// A skill whose body and optional resources are inlined in the manifest.
    Content(ResolvedContextSkillContent),
}

/// Resolved data for a skill baked in from a directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContextSkillSource {
    pub dir: String,
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
