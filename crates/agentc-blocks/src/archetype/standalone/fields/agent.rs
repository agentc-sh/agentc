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
