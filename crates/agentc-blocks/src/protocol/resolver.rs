// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde_json::Value;
use std::collections::HashMap;

use crate::{
    context::ResolvedContext,
    errors::BlocksError,
    protocol::{
        traits::{ErasedProtocol, Protocol},
        types::ResolvedProtocol,
    },
};

pub struct ProtocolResolver {
    protocols: HashMap<String, Box<dyn ErasedProtocol>>,
}

impl ProtocolResolver {
    pub fn new(protocols: HashMap<String, Box<dyn ErasedProtocol>>) -> Self {
        Self { protocols }
    }

    pub fn builder() -> ProtocolResolverBuilder {
        ProtocolResolverBuilder::default()
    }

    pub fn register<T>(&mut self, protocol: T) -> Result<(), BlocksError>
    where
        T: Protocol + 'static,
    {
        let name = protocol.name().to_string();

        if self.protocols.contains_key(&name) {
            return Err(BlocksError::duplicate_registration("protocol", name));
        }

        self.protocols
            .insert(name, Box::new(protocol));

        Ok(())
    }

    pub fn resolve(
        &self,
        protocol_name: &str,
        context: ResolvedContext,
        config: Value,
    ) -> Result<ResolvedProtocol, BlocksError> {
        self.protocols
            .get(protocol_name)
            .ok_or_else(|| BlocksError::UnknownProtocol(protocol_name.to_string()))?
            .resolve_erased(context, config)
    }
}

pub struct ProtocolResolverBuilder {
    error: Option<BlocksError>,
    protocols: HashMap<String, Box<dyn ErasedProtocol>>,
}

impl ProtocolResolverBuilder {
    pub fn new() -> Self {
        Self {
            error: None,
            protocols: HashMap::new(),
        }
    }

    pub fn with_protocol<T>(mut self, protocol: T) -> Self
    where
        T: Protocol + 'static,
    {
        let name = protocol.name().to_string();

        if self.protocols.contains_key(&name) {
            self.error = Some(BlocksError::duplicate_registration("protocol", name));
            return self;
        }

        self.protocols
            .insert(name, Box::new(protocol));

        self
    }

    pub fn build(self) -> Result<ProtocolResolver, BlocksError> {
        if let Some(error) = self.error {
            return Err(error);
        }

        Ok(ProtocolResolver::new(self.protocols))
    }
}

impl Default for ProtocolResolverBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use super::*;
    use crate::composition::GenerationContribution;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    struct TestProtocolConfig {
        path: String,
    }

    struct TestProtocol;

    impl Protocol for TestProtocol {
        type Config = TestProtocolConfig;

        fn name(&self) -> &str {
            "test"
        }

        fn resolve(
            &self,
            _context: ResolvedContext,
            config: Self::Config,
        ) -> Result<ResolvedProtocol, BlocksError> {
            if config.path.is_empty() {
                return Err(BlocksError::invalid("protocol path cannot be empty"));
            }

            Ok(ResolvedProtocol {
                name: Protocol::name(self).to_string(),
                contribution: GenerationContribution::new(),
            })
        }
    }

    fn context() -> ResolvedContext {
        serde_json::from_value(json!({
            "slug": "assistant",
            "agent_name": "assistant",
            "runtime": {
                "default_tenant_id": "default"
            },
            "providers": [],
            "agent": {
                "version": "0.1.0",
                "description": null,
                "prompt": null,
                "capabilities": null,
                "capability_policy": null,
                "model": {
                    "provider": "anthropic",
                    "name": "claude"
                }
            },
            "blocks": {},
            "tools": {},
            "skills": {},
            "http_server": null
        }))
        .unwrap()
    }

    #[test]
    fn builder_rejects_duplicate_protocol_registration() {
        let result = ProtocolResolver::builder()
            .with_protocol(TestProtocol)
            .with_protocol(TestProtocol)
            .build();

        assert!(matches!(
            result,
            Err(BlocksError::DuplicateRegistration {
                component: "protocol",
                ..
            })
        ));
    }

    #[test]
    fn resolver_rejects_unknown_protocol() {
        let resolver = ProtocolResolver::builder()
            .with_protocol(TestProtocol)
            .build()
            .unwrap();

        let result = resolver
            .resolve("missing", context(), json!({}));

        assert!(matches!(result, Err(BlocksError::UnknownProtocol(_))));
    }

    #[test]
    fn resolver_dispatches_typed_config() {
        let resolver = ProtocolResolver::builder()
            .with_protocol(TestProtocol)
            .build()
            .unwrap();
        let protocol = resolver
            .resolve("test", context(), json!({ "path": "/ag-ui" }))
            .unwrap();

        assert_eq!(protocol.name, "test");
    }
}
