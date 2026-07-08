// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};
use futures::Stream;
use tokio_util::sync::{
    CancellationToken,
    DropGuard,
};

/// Cancels a token when the wrapped stream is dropped.
pub struct CancelOnDropStream<S> {
    inner: S,
    _guard: DropGuard,
}

impl<S> CancelOnDropStream<S> {
    pub fn new(
        inner: S,
        token: CancellationToken,
    ) -> Self {
        Self {
            inner,
            _guard: token.drop_guard(),
        }
    }

    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S> Stream for CancelOnDropStream<S>
where
    S: Stream + Unpin,
{
    type Item = S::Item;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}
