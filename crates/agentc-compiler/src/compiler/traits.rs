// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;

use crate::compiler::{
    errors::CompilerError,
    types::{Artifact, CompileParams},
};

#[async_trait]
pub trait Compiler: Send + Sync {
    async fn compile(
        &self,
        params: CompileParams,
        output_sink: &dyn OutputSink,
    ) -> Result<Artifact, CompilerError>;
}

#[async_trait]
pub trait OutputSink: Send + Sync {
    async fn stdout(&self, line: &str);
    async fn stderr(&self, line: &str);
}

pub struct NullOutputSink;

#[async_trait]
impl OutputSink for NullOutputSink {
    async fn stdout(&self, _line: &str) {}
    async fn stderr(&self, _line: &str) {}
}
