// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    archetype::standalone::fields::spec::{FieldsSpec, IntoFieldSpecs},
    context::ResolvedContextHttpServer,
};

impl IntoFieldSpecs for ResolvedContextHttpServer {
    fn extend_fields(&self, fields: &mut FieldsSpec) {
        fields.push(&["server", "host"], &self.host);
        fields.push(&["server", "port"], &self.port);
    }
}
