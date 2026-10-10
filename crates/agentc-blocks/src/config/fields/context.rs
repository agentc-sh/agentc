// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    config::fields::spec::{FieldsSpec, IntoFieldSpecs},
    context::ResolvedContext,
};

impl IntoFieldSpecs for ResolvedContext {
    fn extend_fields(&self, fields: &mut FieldsSpec) {
        fields.extend_from(&self.runtime);

        for provider in self.providers.values() {
            fields.extend_from(provider);
        }

        fields.extend_from(&self.agent);

        for tool in self.tools.values() {
            fields.extend_from(tool);
        }

        if let Some(http) = &self.http_server {
            fields.extend_from(http);
        }
    }
}
