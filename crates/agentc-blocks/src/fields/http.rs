// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    context::ResolvedContextHttpServer,
    fields::spec::{FieldsSpec, IntoFieldSpecs},
};

impl IntoFieldSpecs for ResolvedContextHttpServer {
    fn extend_fields(&self, fields: &mut FieldsSpec) {
        fields.push(&["server", "host"], &self.host);
        fields.push(&["server", "port"], &self.port);
        fields.push(&["server", "max_request_size"], &self.max_request_size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RuntimeValue;

    #[test]
    fn registers_host_and_port() {
        let server = ResolvedContextHttpServer {
            host: RuntimeValue::constant("0.0.0.0".to_string()),
            port: RuntimeValue::constant(8080u16),
            max_request_size: RuntimeValue::constant(2097152usize),
            protocols: vec![],
        };

        let fields = FieldsSpec::collect_from(&server);

        assert!(
            fields
                .get(&["server", "host"])
                .is_some()
        );
        assert!(
            fields
                .get(&["server", "port"])
                .is_some()
        );
        assert!(
            fields
                .get(&["server", "max_request_size"])
                .is_some()
        );
    }
}
