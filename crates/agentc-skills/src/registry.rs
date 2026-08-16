// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_fs::{Fs, embedded::EmbeddedFs, memory::MemoryFs, readonly::ReadOnlyFs};
use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{Value, json};
use std::path::Path;
use tokio::fs::read_dir;

use agentc_prompt::vars::{TemplateVars, TemplateVarsError};

use crate::{
    errors::SkillError,
    skill::{SKILL_FILENAME, Skill},
};

pub const SKILLS_ROOT: &str = "/skills";

/// A registry of available skills, keyed by name.
///
/// Built via [`SkillRegistryBuilder`]. Once constructed the registry is
/// immutable. Wrap it in an [`Arc`] to share it across tool registration and
/// the template vars contributor.
pub struct SkillRegistry {
    skills: IndexMap<String, Skill>,
    fs: Fs,
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

    pub fn fs(&self) -> Fs {
        self.fs.clone()
    }

    pub async fn read_file(&self, skill_name: &str, rel_path: &str) -> Result<String, SkillError> {
        if !self.skills.contains_key(skill_name) {
            return Err(SkillError::resource_not_found(skill_name, rel_path));
        }

        Ok(self
            .fs
            .root()
            .open_file(format!("/{skill_name}/{rel_path}"))
            .await?
            .read_to_string()
            .await?)
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
pub struct SkillRegistryBuilder {
    fs: Fs,
    skills: IndexMap<String, Skill>,
}

impl SkillRegistryBuilder {
    fn mount_skill(
        mut self,
        skill: Skill,
        files: Vec<(String, String)>,
    ) -> Result<Self, SkillError> {
        if self.skills.contains_key(&skill.name) {
            return Ok(self);
        }

        let mut memory = MemoryFs::builder();

        for (path, content) in files {
            memory = memory.file(format!("/{path}"), content);
        }

        self.fs
            .mount(format!("/{}", skill.name), ReadOnlyFs::new(memory.build()?))?;
        self.skills
            .insert(skill.name.clone(), skill);

        Ok(self)
    }

    /// Add a skill from static content baked in at compile time.
    ///
    /// `skill_md` is the full content of the `SKILL.md` file. `resources` is a
    /// slice of `(relative_path, file_content)` pairs covering every file in
    /// the skill directory.
    ///
    /// Returns `Err` if the skill cannot be parsed. If a skill with the same
    /// name is already registered, the new one is silently ignored.
    pub fn with_static(
        self,
        skill_md: &str,
        resources: &[(&str, &str)],
    ) -> Result<Self, SkillError> {
        self.mount_skill(
            Skill::parse(
                skill_md,
                "",
                resources
                    .iter()
                    .map(|(path, _)| path.to_string())
                    .collect(),
            )?,
            std::iter::once((SKILL_FILENAME.to_string(), skill_md.to_string()))
                .chain(
                    resources
                        .iter()
                        .map(|(path, content)| (path.to_string(), content.to_string())),
                )
                .collect(),
        )
    }

    pub async fn with_embedded(self, embedded: EmbeddedFs) -> Result<Self, SkillError> {
        let (skill, files) = Skill::load_fs(&Fs::new(embedded).root()).await?;

        self.mount_skill(skill, files)
    }

    pub async fn with_dir(mut self, dir: &Path) -> Result<Self, SkillError> {
        let mut read_dir = read_dir(dir)
            .await
            .map_err(|e| SkillError::io_error(dir.display().to_string(), e))?;

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let skill_dir = entry.path();
            if !skill_dir.is_dir() || !skill_dir.join(SKILL_FILENAME).exists() {
                continue;
            }

            let (skill, files) = Skill::load_host(&skill_dir).await?;

            self = self.mount_skill(skill, files)?;
        }

        Ok(self)
    }

    /// Build the [`SkillRegistry`].
    pub fn build(self) -> SkillRegistry {
        SkillRegistry { skills: self.skills, fs: self.fs }
    }
}

impl Default for SkillRegistryBuilder {
    fn default() -> Self {
        SkillRegistryBuilder { fs: Fs::empty(), skills: IndexMap::new() }
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

    #[tokio::test]
    async fn with_static_registers_resources_in_the_filesystem() {
        let registry = SkillRegistryBuilder::default()
            .with_static(SKILL_A, &[("scripts/run.sh", "#!/bin/bash\necho hi")])
            .unwrap()
            .build();

        assert_eq!(
            registry
                .read_file("skill-a", "scripts/run.sh")
                .await
                .unwrap(),
            "#!/bin/bash\necho hi"
        );
        assert_eq!(
            registry
                .get("skill-a")
                .unwrap()
                .resources,
            vec!["scripts/run.sh"]
        );
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

    #[tokio::test]
    async fn grafted_filesystem_exposes_skill_resources() {
        let registry = SkillRegistryBuilder::default()
            .with_static(SKILL_A, &[("scripts/run.sh", "#!/bin/bash\necho hi")])
            .unwrap()
            .build();
        let process = Fs::builder()
            .mount("/", MemoryFs::new())
            .build()
            .unwrap();

        process
            .mount_fs(SKILLS_ROOT, registry.fs())
            .unwrap();

        assert_eq!(
            process
                .root()
                .open_file("/skills/skill-a/scripts/run.sh")
                .await
                .unwrap()
                .read_to_string()
                .await
                .unwrap(),
            "#!/bin/bash\necho hi"
        );
    }

    #[tokio::test]
    async fn grafted_skill_resources_are_readonly() {
        let registry = SkillRegistryBuilder::default()
            .with_static(SKILL_A, &[("scripts/run.sh", "#!/bin/bash\necho hi")])
            .unwrap()
            .build();
        let process = Fs::builder()
            .mount("/", MemoryFs::new())
            .build()
            .unwrap();

        process
            .mount_fs(SKILLS_ROOT, registry.fs())
            .unwrap();

        assert!(
            process
                .root()
                .options()
                .write(true)
                .open("/skills/skill-a/scripts/run.sh")
                .await
                .is_err()
        );
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
