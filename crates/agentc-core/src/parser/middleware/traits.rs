// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::parser::errors::ParserError;

pub trait FormatMiddleware<I>: Send + Sync {
    fn apply(&self, input: I) -> Result<I, ParserError>;
}

impl<I, F> FormatMiddleware<I> for F
where
    F: Fn(I) -> Result<I, ParserError> + Send + Sync,
{
    fn apply(&self, input: I) -> Result<I, ParserError> {
        self(input)
    }
}
