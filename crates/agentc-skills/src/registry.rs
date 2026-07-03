// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{Value, json};
use std::path::Path;
use tokio::fs::read_dir;

use agentc_prompt::vars::{TemplateVars, TemplateVarsError};

use crate::{errors::SkillError, skill::Skill};

/// A registry of available skills, keyed by name.
///
/// Built via [`SkillRegistryBuilder`]. Once constructed the registry is
/// immutable. Wrap it in an [`Arc`] to share it across tool registration and
/// the template vars contributor.
pub struct SkillRegistry {
    skills: IndexMap<String, Skill>,
}

impl SkillRegistry {
    /// Returns a builder for constructing a [`SkillRegistry`].
    pub fn builder() -> SkillRegistryBuilder {
        SkillRegistryBuilder::default()
    }

    /// Look up a skill by name.
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// Iterate all skills in insertion order.
    pub fn all(&self) -> impl Iterator<Item = &Skill> {
        self.skills.values()
    }

    /// Returns true when no skills are registered.
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
}

#[async_trait]
impl TemplateVars for SkillRegistry {
    /// Contributes a `skills` variable to the prompt template context.
    ///
    /// The value is an array of objects with `name` and `description` fields,
    /// one entry per registered skill. An empty array is returned when no
    /// skills are registered.
    async fn template_vars(&self) -> Result<Value, TemplateVarsError> {
        Ok(json!({
            "skills": self.all()
                .map(|s| json!({ "name": s.name, "description": s.description }))
                .collect::<Vec<_>>()
        }))
    }
}

/// Builder for [`SkillRegistry`].
#[derive(Default)]
pub struct SkillRegistryBuilder {
    skills: IndexMap<String, Skill>,
}

impl SkillRegistryBuilder {
    /// Add a skill from static content baked in at compile time.
    ///
    /// `skill_md` is the full content of the `SKILL.md` file. `resources` is a
    /// slice of `(relative_path, file_content)` pairs covering every file in
    /// the skill directory.
    ///
    /// Returns `Err` if the skill cannot be parsed. If a skill with the same
    /// name is already registered, the new one is silently ignored.
    pub fn with_static(
        mut self,
        skill_md: &str,
        resources: &[(&str, &str)],
    ) -> Result<Self, SkillError> {
        let skill = Skill::parse(
            skill_md,
            "",
            None,
            resources
                .iter()
                .map(|(p, _)| p.to_string())
                .collect(),
            resources
                .iter()
                .map(|(p, c)| (p.to_string(), c.to_string()))
                .collect(),
        )?;

        self.skills
            .entry(skill.name.clone())
            .or_insert(skill);

        Ok(self)
    }

    /// Scan a directory for skill subdirectories and add them to this builder.
    ///
    /// Each direct subdirectory containing a `SKILL.md` file is loaded via
    /// [`Skill::load`]. Skills that cannot be loaded or parsed are silently
    /// skipped. If a skill with the same name is already registered, the new
    /// one is silently ignored.
    ///
    /// Returns `Err` if the directory itself cannot be read.
    pub async fn with_dir(mut self, dir: &Path) -> Result<Self, SkillError> {
        let mut read_dir = read_dir(dir)
            .await
            .map_err(|e| SkillError::io_error(dir.display().to_string(), e))?;

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let skill_dir = entry.path();
            if !skill_dir.is_dir() || !skill_dir.join("SKILL.md").exists() {
                continue;
            }

            if let Ok(skill) = Skill::load(&skill_dir).await {
                self.skills
                    .entry(skill.name.clone())
                    .or_insert(skill);
            }
        }

        Ok(self)
    }

    /// Merge another [`SkillRegistryBuilder`] into this one.
    ///
    /// Skills already present in this builder take precedence; skills from
    /// `other` are only inserted when their name is not already registered.
    pub fn merge(mut self, other: SkillRegistryBuilder) -> Self {
        for (name, skill) in other.skills {
            self.skills.entry(name).or_insert(skill);
        }
        self
    }

    /// Build the [`SkillRegistry`].
    pub fn build(self) -> SkillRegistry {
        SkillRegistry { skills: self.skills }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentc_prompt::vars::TemplateVars;

    const SKILL_A: &str = "---\nname: skill-a\ndescription: Skill A.\n---\nBody A.";
    const SKILL_B: &str = "---\nname: skill-b\ndescription: Skill B.\n---\nBody B.";

    fn registry_with_a_and_b() -> SkillRegistry {
        SkillRegistryBuilder::default()
            .with_static(SKILL_A, &[])
            .unwrap()
            .with_static(SKILL_B, &[])
            .unwrap()
            .build()
    }

    // -------------------------------------------------------------------------
    // SkillRegistryBuilder::with_static
    // -------------------------------------------------------------------------

    #[test]
    fn with_static_registers_skill() {
        let registry = SkillRegistryBuilder::default()
            .with_static(SKILL_A, &[])
            .unwrap()
            .build();

        let skill = registry.get("skill-a").unwrap();
        assert_eq!(skill.name, "skill-a");
        assert_eq!(skill.description, "Skill A.");
    }

    #[test]
    fn with_static_registers_resource_content() {
        let registry = SkillRegistryBuilder::default()
            .with_static(SKILL_A, &[("scripts/run.sh", "#!/bin/bash\necho hi")])
            .unwrap()
            .build();

        let skill = registry.get("skill-a").unwrap();
        assert_eq!(
            skill
                .resource_content
                .get("scripts/run.sh")
                .map(String::as_str),
            Some("#!/bin/bash\necho hi")
        );
        assert_eq!(skill.resources, vec!["scripts/run.sh"]);
    }

    #[test]
    fn with_static_duplicate_name_is_silently_ignored() {
        let updated = "---\nname: skill-a\ndescription: Updated.\n---\nNew body.";
        let registry = SkillRegistryBuilder::default()
            .with_static(SKILL_A, &[])
            .unwrap()
            .with_static(updated, &[])
            .unwrap()
            .build();

        // The original is kept; the second registration is ignored.
        assert_eq!(
            registry
                .get("skill-a")
                .unwrap()
                .description,
            "Skill A."
        );
    }

    #[test]
    fn with_static_invalid_skill_returns_err() {
        let result = SkillRegistryBuilder::default().with_static("no frontmatter", &[]);
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // SkillRegistryBuilder::merge
    // -------------------------------------------------------------------------

    #[test]
    fn merge_non_overlapping_contains_all_skills() {
        let a = SkillRegistryBuilder::default()
            .with_static(SKILL_A, &[])
            .unwrap();
        let b = SkillRegistryBuilder::default()
            .with_static(SKILL_B, &[])
            .unwrap();
        let registry = a.merge(b).build();

        assert!(registry.get("skill-a").is_some());
        assert!(registry.get("skill-b").is_some());
    }

    #[test]
    fn merge_existing_skill_takes_precedence() {
        let original = "---\nname: skill-a\ndescription: Original.\n---\nBody.";
        let replacement = "---\nname: skill-a\ndescription: Replacement.\n---\nBody.";

        let base = SkillRegistryBuilder::default()
            .with_static(original, &[])
            .unwrap();
        let other = SkillRegistryBuilder::default()
            .with_static(replacement, &[])
            .unwrap();
        let registry = base.merge(other).build();

        assert_eq!(
            registry
                .get("skill-a")
                .unwrap()
                .description,
            "Original."
        );
    }

    // -------------------------------------------------------------------------
    // SkillRegistry
    // -------------------------------------------------------------------------

    #[test]
    fn is_empty_on_empty_builder() {
        assert!(
            SkillRegistryBuilder::default()
                .build()
                .is_empty()
        );
    }

    #[test]
    fn is_empty_false_after_registration() {
        let registry = SkillRegistryBuilder::default()
            .with_static(SKILL_A, &[])
            .unwrap()
            .build();
        assert!(!registry.is_empty());
    }

    #[test]
    fn get_returns_none_for_unknown_name() {
        assert!(
            registry_with_a_and_b()
                .get("does-not-exist")
                .is_none()
        );
    }

    #[test]
    fn all_returns_all_registered_skills() {
        let registry = registry_with_a_and_b();
        let names: Vec<&str> = registry
            .all()
            .map(|s| s.name.as_str())
            .collect();
        assert!(names.contains(&"skill-a"));
        assert!(names.contains(&"skill-b"));
        assert_eq!(names.len(), 2);
    }

    // -------------------------------------------------------------------------
    // SkillRegistry::template_vars
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn template_vars_contains_skills_array() {
        let registry = registry_with_a_and_b();
        let vars = registry.template_vars().await.unwrap();

        let skills = vars
            .get("skills")
            .and_then(|v| v.as_array())
            .unwrap();
        assert_eq!(skills.len(), 2);

        let names: Vec<&str> = skills
            .iter()
            .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
            .collect();
        assert!(names.contains(&"skill-a"));
        assert!(names.contains(&"skill-b"));
    }

    #[tokio::test]
    async fn template_vars_entries_have_name_and_description() {
        let registry = SkillRegistryBuilder::default()
            .with_static(SKILL_A, &[])
            .unwrap()
            .build();
        let vars = registry.template_vars().await.unwrap();

        let entry = &vars["skills"][0];
        assert_eq!(entry["name"], "skill-a");
        assert_eq!(entry["description"], "Skill A.");
    }

    #[tokio::test]
    async fn template_vars_empty_registry_produces_empty_array() {
        let registry = SkillRegistryBuilder::default().build();
        let vars = registry.template_vars().await.unwrap();

        let skills = vars
            .get("skills")
            .and_then(|v| v.as_array())
            .unwrap();
        assert!(skills.is_empty());
    }
}
