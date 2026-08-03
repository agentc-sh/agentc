// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use convert_case::{Case, Casing};
use minijinja::{Environment, context};

use agentc_compiler::generator::vfs::VirtualFileSystem;

use crate::init::errors::InitError;

const PY_PYPROJECT: &str = include_str!("templates/python/pyproject.toml");
const PY_README: &str = include_str!("templates/python/README.md");
const PY_TOOL_INIT: &str = include_str!("templates/python/tool_init.py");

const JS_PACKAGE: &str = include_str!("templates/javascript/package.json");
const JS_TSCONFIG: &str = include_str!("templates/javascript/tsconfig.json");
const JS_README: &str = include_str!("templates/javascript/README.md");
const JS_INDEX: &str = include_str!("templates/javascript/src/index.ts");
const JS_PNPM_WORKSPACE: &str = include_str!("templates/javascript/pnpm-workspace.yaml");

pub enum ToolLanguage {
    Python,
    Javascript,
}

pub struct InitToolParams {
    pub name: String,
    pub language: ToolLanguage,
}

pub struct InitTool;

impl InitTool {
    fn render_template(src: &str, ctx: &minijinja::Value, file: &str) -> Result<String, InitError> {
        Environment::new()
            .render_str(src, ctx)
            .map_err(|e| InitError::TemplateFailed { file: file.to_string(), source: e })
    }

    pub fn scaffold(params: InitToolParams) -> Result<VirtualFileSystem, InitError> {
        let ctx = context! {
            name => params.name,
            name_kebab => params.name.to_case(Case::Kebab),
            name_snake => params.name.to_case(Case::Snake),
            name_pascal => params.name.to_case(Case::Pascal),
        };

        let mut vfs = VirtualFileSystem::new();

        match params.language {
            ToolLanguage::Python => {
                vfs.insert(
                    "pyproject.toml",
                    Self::render_template(PY_PYPROJECT, &ctx, "pyproject.toml")?,
                );
                vfs.insert("README.md", Self::render_template(PY_README, &ctx, "README.md")?);
                vfs.insert(
                    format!("{}/__init__.py", params.name.to_case(Case::Snake)),
                    Self::render_template(PY_TOOL_INIT, &ctx, "__init__.py")?,
                );
                vfs.insert(format!("{}/py.typed", params.name.to_case(Case::Snake)), String::new());
            }
            ToolLanguage::Javascript => {
                vfs.insert(
                    "package.json",
                    Self::render_template(JS_PACKAGE, &ctx, "package.json")?,
                );
                vfs.insert(
                    "tsconfig.json",
                    Self::render_template(JS_TSCONFIG, &ctx, "tsconfig.json")?,
                );
                vfs.insert("README.md", Self::render_template(JS_README, &ctx, "README.md")?);
                vfs.insert("src/index.ts", Self::render_template(JS_INDEX, &ctx, "src/index.ts")?);
                vfs.insert(
                    "pnpm-workspace.yaml",
                    Self::render_template(JS_PNPM_WORKSPACE, &ctx, "pnpm-workspace.yaml")?,
                );
            }
        }

        Ok(vfs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_pyproject_contains_snake_name() {
        let vfs = InitTool::scaffold(InitToolParams {
            name: "my_tool".into(),
            language: ToolLanguage::Python,
        })
        .unwrap();
        let content = vfs.get("pyproject.toml").unwrap();
        assert!(content.contains("name = \"my_tool\""), "pyproject.toml missing name: {content}");
    }

    #[test]
    fn python_init_py_contains_pascal_class() {
        let vfs = InitTool::scaffold(InitToolParams {
            name: "my_tool".into(),
            language: ToolLanguage::Python,
        })
        .unwrap();
        let content = vfs.get("my_tool/__init__.py").unwrap();
        assert!(content.contains("class MyTool"), "__init__.py missing class MyTool: {content}");
    }

    #[test]
    fn python_py_typed_is_present() {
        let vfs = InitTool::scaffold(InitToolParams {
            name: "my_tool".into(),
            language: ToolLanguage::Python,
        })
        .unwrap();
        assert!(vfs.get("my_tool/py.typed").is_some());
    }

    #[test]
    fn javascript_package_json_contains_kebab_name() {
        let vfs = InitTool::scaffold(InitToolParams {
            name: "my-tool".into(),
            language: ToolLanguage::Javascript,
        })
        .unwrap();
        let content = vfs.get("package.json").unwrap();
        assert!(
            content.contains("\"name\": \"my-tool\""),
            "package.json missing name: {content}"
        );
    }

    #[test]
    fn javascript_index_ts_contains_snake_export() {
        let vfs = InitTool::scaffold(InitToolParams {
            name: "my-tool".into(),
            language: ToolLanguage::Javascript,
        })
        .unwrap();
        let content = vfs.get("src/index.ts").unwrap();
        assert!(content.contains("export const my_tool"), "index.ts missing export: {content}");
    }

    #[test]
    fn javascript_tsconfig_is_present() {
        let vfs = InitTool::scaffold(InitToolParams {
            name: "my-tool".into(),
            language: ToolLanguage::Javascript,
        })
        .unwrap();
        assert!(vfs.get("tsconfig.json").is_some());
    }

    #[test]
    fn javascript_pnpm_workspace_allowlists_build_scripts() {
        let vfs = InitTool::scaffold(InitToolParams {
            name: "my-tool".into(),
            language: ToolLanguage::Javascript,
        })
        .unwrap();
        let content = vfs.get("pnpm-workspace.yaml").unwrap();
        assert!(content.contains("esbuild"), "pnpm-workspace.yaml missing esbuild: {content}");
        assert!(
            content.contains("@agentc-sh/tdk"),
            "pnpm-workspace.yaml missing tdk: {content}"
        );
    }
}
