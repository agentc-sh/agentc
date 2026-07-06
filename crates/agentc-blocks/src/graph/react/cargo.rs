// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_compiler::generator::{
    blocks::template::TemplateFragment, context::GenerationContext, errors::GeneratorError,
    extension::ExtensionRegistry,
};

use crate::context::ResolvedContext;

pub struct ReActCargoFragment {
    pub has_ag_ui: bool,
}

impl TemplateFragment<ResolvedContext> for ReActCargoFragment {
    fn generate_contribution(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        point: &str,
    ) -> Result<String, GeneratorError> {
        let version = env!("CARGO_PKG_VERSION");

        match point {
            "cargo::dependencies" => {
                if self.has_ag_ui {
                    Ok(format!(
                        "agentc-agent-react = {{ version = \"{version}\", features = [\"ag-ui\"] }}"
                    ))
                } else {
                    Ok(format!(
                        "agentc-agent-react = {{ version = \"{version}\" }}"
                    ))
                }
            }
            "cargo::patches" => {
                Ok("agentc-agent-react = { path = \"../runtime/agentc-agent-react\" }".to_string())
            }
            _ => Err(GeneratorError::unexpected(format!("Unknown extension point '{}'", point))),
        }
    }

    fn generate_files(
        &self,
        _ctx: &GenerationContext<ResolvedContext>,
        _registry: &ExtensionRegistry,
    ) -> Result<Vec<(std::path::PathBuf, String)>, GeneratorError> {
        Ok(vec![])
    }
}
