// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use futures::{
    StreamExt,
    stream::{Stream, empty, once},
    task::AtomicWaker,
};
use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
};

use crate::{errors::ModelError, types::stream::CompletionStreamEvent};

/// Shared pause state between a [`ChatCompletionStream`] and all
/// [`PauseHandle`] clones derived from it.
struct PauseState {
    paused: AtomicBool,
    waker: AtomicWaker,
}

impl PauseState {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            paused: AtomicBool::new(false),
            waker: AtomicWaker::new(),
        })
    }
}

/// A handle for pausing and resuming a [`ChatCompletionStream`] from
/// outside the stream itself.
#[derive(Clone)]
pub struct PauseHandle {
    state: Arc<PauseState>,
}

impl PauseHandle {
    /// Pause event delivery. The inner stream continues to be polled
    /// internally but events are held until [`PauseHandle::resume`] is called.
    pub fn pause(&self) {
        self.state
            .paused
            .store(true, Ordering::Release);
    }

    /// Resume event delivery and wake the stream's task.
    pub fn resume(&self) {
        self.state
            .paused
            .store(false, Ordering::Release);
        self.state.waker.wake();
    }

    /// Whether the stream is currently paused.
    pub fn is_paused(&self) -> bool {
        self.state
            .paused
            .load(Ordering::Acquire)
    }
}

/// A streaming chat completion response. Implements [`Stream`] so it
/// works with [`StreamExt`](futures::StreamExt), `while let`, and any
/// async runtime.
///
/// Obtain a [`PauseHandle`] via [`ChatCompletionStream::pause_handle`]
/// to pause and resume event delivery from outside the stream.
pub struct ChatCompletionStream {
    inner: Pin<Box<dyn Stream<Item = Result<CompletionStreamEvent, ModelError>> + Send>>,
    pause_state: Arc<PauseState>,
}

impl ChatCompletionStream {
    /// Construct from any stream of events. Used by provider implementations
    /// inside [`CompletionModel::complete`](crate::traits::CompletionModel::complete).
    pub fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<CompletionStreamEvent, ModelError>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
            pause_state: PauseState::new(),
        }
    }

    /// Drive `stream` to its first item so the provider handshake completes
    /// here rather than on the consumer's first poll. A first item that is an
    /// error (a non-2xx status or connection failure surfaced by the provider)
    /// is returned as the handshake result; otherwise it is re-emitted at the
    /// head of the returned stream so no events are lost.
    pub async fn establish<S>(stream: S) -> Result<Self, ModelError>
    where
        S: Stream<Item = Result<CompletionStreamEvent, ModelError>> + Send + 'static,
    {
        let mut stream = Box::pin(stream);

        match stream.next().await {
            Some(Err(error)) => Err(error),
            Some(Ok(first)) => Ok(Self::new(once(async move { Ok(first) }).chain(stream))),
            None => Ok(Self::new(empty())),
        }
    }

    /// Returns a [`PauseHandle`] that shares pause state with this stream.
    pub fn pause_handle(&self) -> PauseHandle {
        PauseHandle { state: self.pause_state.clone() }
    }
}

impl Stream for ChatCompletionStream {
    type Item = Result<CompletionStreamEvent, ModelError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self
            .pause_state
            .paused
            .load(Ordering::Acquire)
        {
            // Register the waker so resume() can wake this task.
            self.pause_state
                .waker
                .register(cx.waker());

            // Re-check after registering to close the race between the
            // initial load and the register call.
            if self
                .pause_state
                .paused
                .load(Ordering::Acquire)
            {
                return Poll::Pending;
            }
        }

        self.inner.as_mut().poll_next(cx)
    }
}
