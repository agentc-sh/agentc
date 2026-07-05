// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    archetype::standalone::fields::spec::{FieldsSpec, IntoFieldSpecs},
    context::ResolvedContextRuntime,
};

impl IntoFieldSpecs for ResolvedContextRuntime {
    fn extend_fields(&self, fields: &mut FieldsSpec) {
        fields.push(&["default_tenant_id"], &self.default_tenant_id);
    }
}
