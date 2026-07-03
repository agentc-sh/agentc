// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[allow(unused_extern_crates)]
extern crate self as agentc_skills;

pub mod builder;
pub mod errors;
pub mod registry;
pub mod skill;
pub mod tools;

pub mod prelude {
    pub use crate::builder::{AgentBuilderSkillsExt, ToolRegistryBuilderSkillsExt};
    pub use crate::errors::SkillError;
    pub use crate::registry::{SkillRegistry, SkillRegistryBuilder};
    pub use crate::skill::{AllowedTool, Skill, SkillInfo};
    pub use crate::tools::run::MaterializationPolicy;
}
