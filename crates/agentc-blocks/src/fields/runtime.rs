// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    context::ResolvedContextRuntime,
    fields::spec::{FieldsSpec, IntoFieldSpecs},
};

impl IntoFieldSpecs for ResolvedContextRuntime {
    fn extend_fields(&self, fields: &mut FieldsSpec) {
        fields.push(&["default_tenant_id"], &self.default_tenant_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RuntimeValue;

    #[test]
    fn registers_the_default_tenant_id() {
        let runtime = ResolvedContextRuntime {
            default_tenant_id: RuntimeValue::constant("public".to_string()),
        };

        let fields = FieldsSpec::collect_from(&runtime);

        assert!(
            fields
                .get(&["default_tenant_id"])
                .is_some()
        );
        assert_eq!(fields.as_inner().len(), 1);
    }
}
