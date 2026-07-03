// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use convert_case::{Case, Casing};
use minijinja::{Environment, context};

use agentc_compiler::generator::vfs::VirtualFileSystem;

use crate::init::errors::InitError;

const TEMPLATE_AGENT_ACL: &str = include_str!("templates/agent/agent.acl");
const TEMPLATE_GITIGNORE: &str = include_str!("templates/agent/gitignore");
const TEMPLATE_README: &str = include_str!("templates/agent/README.md");

pub struct InitAgentParams {
    pub name: String,
}

pub struct InitAgent;

impl InitAgent {
    fn render_template(src: &str, ctx: &minijinja::Value, file: &str) -> Result<String, InitError> {
        Environment::new()
            .render_str(src, ctx)
            .map_err(|e| InitError::TemplateFailed { file: file.to_string(), source: e })
    }

    pub fn scaffold(params: InitAgentParams) -> Result<VirtualFileSystem, InitError> {
        let ctx = context! {
            name => params.name,
            name_kebab => params.name.to_case(Case::Kebab),
            name_snake => params.name.to_case(Case::Snake),
            name_pascal => params.name.to_case(Case::Pascal),
        };

        let mut vfs = VirtualFileSystem::new();
        vfs.insert("agent.acl", Self::render_template(TEMPLATE_AGENT_ACL, &ctx, "agent.acl")?);
        vfs.insert(".gitignore", Self::render_template(TEMPLATE_GITIGNORE, &ctx, ".gitignore")?);
        vfs.insert("README.md", Self::render_template(TEMPLATE_README, &ctx, "README.md")?);

        Ok(vfs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_acl_contains_name() {
        let vfs = InitAgent::scaffold(InitAgentParams { name: "my_agent".into() }).unwrap();
        let content = vfs.get("agent.acl").unwrap();
        assert!(
            content.contains("agent \"my_agent\""),
            "agent.acl missing agent block: {content}"
        );
    }

    #[test]
    fn gitignore_is_present() {
        let vfs = InitAgent::scaffold(InitAgentParams { name: "my_agent".into() }).unwrap();
        assert!(vfs.get(".gitignore").is_some());
    }

    #[test]
    fn readme_contains_kebab_name() {
        let vfs = InitAgent::scaffold(InitAgentParams { name: "my_agent".into() }).unwrap();
        let content = vfs.get("README.md").unwrap();
        assert!(content.contains("my-agent"), "README.md missing kebab name: {content}");
    }
}
