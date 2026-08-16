// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_fs::fs::{Dir, FileType};
use futures::TryStreamExt;
use regex::Regex;
use sanitizer::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, path::Path};
use tokio::fs::read_dir;
use validator::Validate;

use agentc_agent::types::capability::{Capability, CapabilitySet};

use crate::errors::SkillError;

pub const SKILL_FILENAME: &str = "SKILL.md";

lazy_static::lazy_static! {
    static ref FRONTMATTER_RE: Regex =
        Regex::new(r"^[[:space:]]*\-\-\-\r?\n((?s).*?(?-s))\-\-\-\r?\n((?s).*(?-s))$").unwrap();
}

/// The parsed frontmatter of a SKILL.md file.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, Sanitizer)]
pub struct SkillInfo {
    /// The skill's name. Falls back to the parent directory name if absent.
    #[serde(default)]
    #[sanitizer(trim)]
    pub name: Option<String>,

    /// Human-readable description of the skill's purpose.
    #[validate(length(min = 1))]
    #[sanitizer(trim)]
    pub description: String,

    /// Space-delimited list of agent tools pre-approved for this skill.
    ///
    /// Parsed from the `allowed-tools` frontmatter field. When absent the
    /// skill imposes no tool restrictions.
    #[serde(rename = "allowed-tools", default)]
    pub allowed_tools: Option<String>,

    /// Any additional frontmatter fields beyond the known fields above.
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// A parsed entry from the `allowed-tools` frontmatter field.
///
/// Raw entries follow the spec format: `ToolName` or `ToolName(pattern)`,
/// for example `Bash`, `Read`, or `Bash(git:*)`.
#[derive(Debug, Clone)]
pub struct AllowedTool {
    /// The tool name, e.g. `"Bash"` or `"Read"`.
    pub name: String,
    /// The optional constraint pattern inside parentheses, e.g. `"git:*"`.
    pub pattern: Option<String>,
}

impl AllowedTool {
    /// Parse a single raw `allowed-tools` entry.
    ///
    /// Returns `None` for empty or malformed entries so callers can use
    /// `filter_map` to skip them silently.
    pub fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();

        if raw.is_empty() {
            return None;
        }

        if let Some(pos) = raw.find('(') {
            let name = raw[..pos].trim().to_string();
            if name.is_empty() {
                return None;
            }
            let pattern = raw[pos + 1..]
                .trim_end_matches(')')
                .trim()
                .to_string();
            Some(Self {
                name,
                pattern: if pattern.is_empty() {
                    None
                } else {
                    Some(pattern)
                },
            })
        } else {
            Some(Self { name: raw.to_string(), pattern: None })
        }
    }

    /// Returns `true` if this entry permits the named tool.
    pub fn permits(&self, tool_name: &str) -> bool {
        self.name == tool_name
    }
}

/// A fully parsed skill.
pub struct Skill {
    /// The skill's unique name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// The markdown body of the SKILL.md with frontmatter stripped.
    pub body: String,
    /// Relative paths to bundled resource files.
    pub resources: Vec<String>,
    /// Frontmatter fields beyond the known fields, exposed by `describe_skill`.
    pub extra_frontmatter: Value,
    /// Tools pre-approved for this skill, as declared in `allowed-tools`.
    ///
    /// Each entry is a raw spec string such as `"Bash(git:*)"` or `"Read"`.
    /// An empty list means the skill declares no tool restrictions.
    pub allowed_tools: Vec<String>,
}

impl Skill {
    fn from_files(
        dir_name: &str,
        files: Vec<(String, String)>,
    ) -> Result<(Skill, Vec<(String, String)>), SkillError> {
        let skill_md = files
            .iter()
            .find(|(path, _)| path == SKILL_FILENAME)
            .map(|(_, content)| content.as_str())
            .ok_or_else(|| SkillError::resource_not_found(dir_name, SKILL_FILENAME))?;

        Ok((
            Skill::parse(
                skill_md,
                dir_name,
                files
                    .iter()
                    .map(|(path, _)| path.clone())
                    .filter(|path| path != SKILL_FILENAME)
                    .collect(),
            )?,
            files,
        ))
    }

    /// Returns the [`CapabilitySet`] implied by this skill's `allowed-tools`
    /// declaration.
    ///
    /// Each spec tool name is mapped to an agentc capability string:
    /// `Bash` to `process::bash`, `Read` to `filesystem::read`,
    /// `Write`/`Edit` to `filesystem::write`, and anything else to
    /// `tools::<lowercase_name>`. Skills with no `allowed-tools` return an
    /// empty set.
    pub fn required_capabilities(&self) -> CapabilitySet {
        CapabilitySet::from(
            self.allowed_tools
                .iter()
                .filter_map(|t| AllowedTool::parse(t))
                .map(|t| match t.name.as_str() {
                    "Bash" => Capability::new("process::bash"),
                    "Read" => Capability::new("filesystem::read"),
                    "Write" | "Edit" => Capability::new("filesystem::write"),
                    other => Capability::new(format!("tools::{}", other.to_lowercase())),
                })
                .collect::<Vec<_>>(),
        )
    }

    pub async fn load_fs(dir: &Dir) -> Result<(Skill, Vec<(String, String)>), SkillError> {
        Skill::from_files(
            "",
            dir.walk()
                .await?
                .try_filter_map(|entry| async move {
                    if entry.file_type() != FileType::File {
                        return Ok(None);
                    }

                    Ok(Some((
                        entry
                            .path()
                            .to_string_lossy()
                            .trim_start_matches('/')
                            .to_string(),
                        dir.open_file(entry.path())
                            .await?
                            .read_to_string()
                            .await?,
                    )))
                })
                .try_collect()
                .await?,
        )
    }

    pub async fn load_host(dir: &Path) -> Result<(Skill, Vec<(String, String)>), SkillError> {
        let mut files = Vec::new();
        let mut stack = vec![dir.to_path_buf()];

        while let Some(current) = stack.pop() {
            let mut entries = read_dir(&current)
                .await
                .map_err(|error| SkillError::io_error(current.display().to_string(), error))?;

            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|error| SkillError::io_error(current.display().to_string(), error))?
            {
                let path = entry.path();

                if path.is_dir() {
                    stack.push(path);

                    continue;
                }

                if !path.is_file() {
                    continue;
                }

                if let Some(relative_path) = path
                    .strip_prefix(dir)
                    .ok()
                    .and_then(|path| path.to_str())
                {
                    files.push((
                        relative_path.to_string(),
                        tokio::fs::read_to_string(&path)
                            .await
                            .map_err(|error| {
                                SkillError::io_error(path.display().to_string(), error)
                            })?,
                    ));
                }
            }
        }

        Skill::from_files(
            dir.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(""),
            files,
        )
    }

    /// Parse a SKILL.md file following the Agent Skills specification.
    ///
    /// `dir_name` is used as a fallback when `name` is absent from the
    /// frontmatter. Pass an empty string for inlined skills with no directory.
    ///
    /// Returns `Err` if the frontmatter is missing, unparseable, or the
    /// description is absent or empty.
    pub fn parse(
        content: &str,
        dir_name: &str,
        resources: Vec<String>,
    ) -> Result<Skill, SkillError> {
        let caps = FRONTMATTER_RE
            .captures(content)
            .ok_or_else(|| SkillError::unparsable_frontmatter("no frontmatter delimiters found"))?;

        let mut info = serde_norway::from_str::<SkillInfo>(&caps[1])
            .map_err(|e| SkillError::unparsable_frontmatter(e.to_string()))?;

        info.sanitize();

        info.validate().map_err(|_| {
            SkillError::missing_description(info.name.as_deref().unwrap_or(dir_name))
        })?;

        let name = info
            .name
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| dir_name.to_string());

        let allowed_tools = info
            .allowed_tools
            .as_deref()
            .map(|s| {
                s.split_whitespace()
                    .map(|t| t.to_string())
                    .collect()
            })
            .unwrap_or_default();

        Ok(Skill {
            name,
            description: info.description,
            body: caps[2].trim().to_string(),
            resources,
            extra_frontmatter: Value::Object(info.extra.into_iter().collect()),
            allowed_tools,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_tool_parse_simple_name() {
        let t = AllowedTool::parse("Read").unwrap();
        assert_eq!(t.name, "Read");
        assert!(t.pattern.is_none());
    }

    #[test]
    fn allowed_tool_parse_with_pattern() {
        let t = AllowedTool::parse("Bash(git:*)").unwrap();
        assert_eq!(t.name, "Bash");
        assert_eq!(t.pattern.as_deref(), Some("git:*"));
    }

    #[test]
    fn allowed_tool_parse_empty_pattern_is_none() {
        let t = AllowedTool::parse("Bash()").unwrap();
        assert_eq!(t.name, "Bash");
        assert!(t.pattern.is_none());
    }

    #[test]
    fn allowed_tool_parse_trims_whitespace() {
        let t = AllowedTool::parse("  Read  ").unwrap();
        assert_eq!(t.name, "Read");
    }

    #[test]
    fn allowed_tool_parse_empty_returns_none() {
        assert!(AllowedTool::parse("").is_none());
        assert!(AllowedTool::parse("   ").is_none());
    }

    #[test]
    fn allowed_tool_parse_empty_name_before_paren_returns_none() {
        assert!(AllowedTool::parse("(git:*)").is_none());
    }

    #[test]
    fn allowed_tool_permits_exact_match() {
        let t = AllowedTool::parse("Bash(git:*)").unwrap();
        assert!(t.permits("Bash"));
        assert!(!t.permits("Read"));
    }

    #[test]
    fn allowed_tool_permits_is_case_sensitive() {
        let t = AllowedTool::parse("Read").unwrap();
        assert!(!t.permits("read"));
        assert!(!t.permits("READ"));
    }

    fn minimal_skill_md() -> &'static str {
        "---\nname: my-skill\ndescription: Does something useful.\n---\nInstructions here."
    }

    #[test]
    fn skill_parse_valid_minimal() {
        let skill = Skill::parse(minimal_skill_md(), "my-skill", vec![]).unwrap();
        assert_eq!(skill.name, "my-skill");
        assert_eq!(skill.description, "Does something useful.");
        assert_eq!(skill.body, "Instructions here.");
        assert!(skill.allowed_tools.is_empty());
    }

    #[test]
    fn skill_parse_name_falls_back_to_dir_name() {
        let content = "---\ndescription: No name in frontmatter.\n---\nBody.";
        let skill = Skill::parse(content, "fallback-name", vec![]).unwrap();
        assert_eq!(skill.name, "fallback-name");
    }

    #[test]
    fn skill_parse_empty_description_errors() {
        // An empty (or whitespace-only) description fails validation after trimming.
        let content = "---\nname: no-desc\ndescription: \"   \"\n---\nBody.";
        assert!(matches!(
            Skill::parse(content, "no-desc", vec![]),
            Err(SkillError::MissingDescription { .. })
        ));
    }

    #[test]
    fn skill_parse_absent_description_errors() {
        // A completely absent description fails deserialization.
        let content = "---\nname: no-desc\n---\nBody.";
        assert!(matches!(
            Skill::parse(content, "no-desc", vec![]),
            Err(SkillError::UnparsableFrontmatter(_))
        ));
    }

    #[test]
    fn skill_parse_missing_frontmatter_errors() {
        let content = "No frontmatter at all.";
        assert!(matches!(
            Skill::parse(content, "x", vec![]),
            Err(SkillError::UnparsableFrontmatter(_))
        ));
    }

    #[test]
    fn skill_parse_allowed_tools_split_correctly() {
        let content = "---\nname: my-skill\ndescription: Desc.\nallowed-tools: Bash(git:*) Read Write\n---\nBody.";
        let skill = Skill::parse(content, "my-skill", vec![]).unwrap();
        assert_eq!(skill.allowed_tools, vec!["Bash(git:*)", "Read", "Write"]);
    }

    #[test]
    fn skill_parse_extra_frontmatter_captured() {
        let content =
            "---\nname: my-skill\ndescription: Desc.\ncompatibility: Requires git\n---\nBody.";
        let skill = Skill::parse(content, "my-skill", vec![]).unwrap();
        assert_eq!(
            skill
                .extra_frontmatter
                .get("compatibility")
                .and_then(|v| v.as_str()),
            Some("Requires git"),
        );
    }

    #[test]
    fn skill_parse_body_trimmed() {
        let content = "---\nname: my-skill\ndescription: Desc.\n---\n\n  Body text.  \n";
        let skill = Skill::parse(content, "my-skill", vec![]).unwrap();
        assert_eq!(skill.body, "Body text.");
    }

    fn skill_with_allowed_tools(tools: &str) -> Skill {
        let content =
            format!("---\nname: s\ndescription: D.\nallowed-tools: {}\n---\nBody.", tools,);
        Skill::parse(&content, "s", vec![]).unwrap()
    }

    #[test]
    fn required_capabilities_empty_when_no_allowed_tools() {
        let skill = Skill::parse(minimal_skill_md(), "s", vec![]).unwrap();
        assert!(
            skill
                .required_capabilities()
                .as_inner()
                .is_empty()
        );
    }

    #[test]
    fn required_capabilities_bash_maps_to_process_bash() {
        let skill = skill_with_allowed_tools("Bash(git:*)");
        assert!(
            skill
                .required_capabilities()
                .has(&Capability::new("process::bash"))
        );
    }

    #[test]
    fn required_capabilities_read_maps_to_filesystem_read() {
        let skill = skill_with_allowed_tools("Read");
        assert!(
            skill
                .required_capabilities()
                .has(&Capability::new("filesystem::read"))
        );
    }

    #[test]
    fn required_capabilities_write_and_edit_map_to_filesystem_write() {
        let skill = skill_with_allowed_tools("Write Edit");
        let caps = skill.required_capabilities();
        assert!(caps.has(&Capability::new("filesystem::write")));
    }

    #[test]
    fn required_capabilities_unknown_tool_maps_to_tools_namespace() {
        let skill = skill_with_allowed_tools("MyTool");
        assert!(
            skill
                .required_capabilities()
                .has(&Capability::new("tools::mytool"))
        );
    }

    #[test]
    fn required_capabilities_skips_malformed_entries() {
        let skill = skill_with_allowed_tools("(bad) Read");
        let caps = skill.required_capabilities();
        // The malformed "(bad)" entry is skipped; only "Read" contributes.
        assert!(caps.has(&Capability::new("filesystem::read")));
        assert_eq!(caps.as_inner().len(), 1);
    }
}
