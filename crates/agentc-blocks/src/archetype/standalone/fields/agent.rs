// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    archetype::standalone::fields::spec::{FieldsSpec, IntoFieldSpecs},
    context::ResolvedContextAgent,
};

impl IntoFieldSpecs for ResolvedContextAgent {
    fn extend_fields(&self, fields: &mut FieldsSpec) {
        fields.push(&["agent", "model", "provider"], &self.model.provider);
        fields.push(&["agent", "model", "name"], &self.model.name);

        if let Some(capabilities) = &self.capabilities {
            fields.push(&["agent", "capabilities"], capabilities);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{context::ResolvedContextAgentModel, types::RuntimeValue};

    fn agent(capabilities: Option<RuntimeValue<Vec<String>>>) -> ResolvedContextAgent {
        ResolvedContextAgent {
            version: "1.0.0".to_string(),
            description: None,
            prompt: None,
            capabilities,
            capability_policy: None,
            model: ResolvedContextAgentModel {
                provider: RuntimeValue::constant("anthropic".to_string()),
                name: RuntimeValue::constant("claude".to_string()),
            },
        }
    }

    #[test]
    fn always_registers_the_model_provider_and_name() {
        let fields = FieldsSpec::collect_from(&agent(None));

        assert!(
            fields
                .get(&["agent", "model", "provider"])
                .is_some()
        );
        assert!(
            fields
                .get(&["agent", "model", "name"])
                .is_some()
        );
        assert!(
            fields
                .get(&["agent", "capabilities"])
                .is_none()
        );
    }

    #[test]
    fn registers_capabilities_only_when_present() {
        let fields = FieldsSpec::collect_from(&agent(Some(RuntimeValue::constant(vec![
            "network".to_string(),
        ]))));

        assert!(
            fields
                .get(&["agent", "capabilities"])
                .is_some()
        );
    }
}
