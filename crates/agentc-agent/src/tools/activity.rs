// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use json_patch::PatchOperation;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter, Result as FmtResult};
use tokio::sync::mpsc;

/// A structured, incremental update emitted by a tool during execution.
///
/// `activity_type` is a free-form string the frontend uses to route the update
/// to the correct UI component (e.g., `"file_search"`, `"code_generation"`).
/// `patch` is a list of RFC 6902 JSON Patch operations the client applies
/// incrementally to its local activity state for this tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityDelta {
    pub activity_type: String,
    pub patch: Vec<PatchOperation>,
}

/// A handle that a tool uses to emit [`ActivityDelta`] values during execution.
///
/// Constructed by [`ToolDispatcher::dispatch`](crate::tools::dispatcher::ToolDispatcher::dispatch)
/// and threaded through [`ToolInput`](crate::tools::input::ToolInput) /
/// [`TypedToolInput`](crate::tools::input::TypedToolInput). Clone is cheap: it
/// clones the inner `Option<Sender>` which is an `Arc` under the hood.
#[derive(Clone)]
pub struct ActivityEmitter {
    tx: Option<mpsc::Sender<ActivityDelta>>,
}

impl ActivityEmitter {
    /// Create an emitter backed by `tx`. Deltas sent to this emitter are
    /// received on the corresponding [`Receiver<ActivityDelta>`](mpsc::Receiver).
    pub fn new(tx: mpsc::Sender<ActivityDelta>) -> Self {
        Self { tx: Some(tx) }
    }

    /// Create a no-op emitter. [`emit`](ActivityEmitter::emit) does nothing.
    /// No channel is allocated. Use this in tests and anywhere activity
    /// emission is not needed.
    pub fn noop() -> Self {
        Self { tx: None }
    }

    /// Emit a delta. Non-blocking: uses `try_send` and silently drops the
    /// delta if the channel is full or the receiver has been dropped.
    pub async fn emit(&self, delta: ActivityDelta) {
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(delta);
        }
    }

    /// Returns a clone of the inner sender, or `None` if this is a noop emitter.
    pub fn sender(&self) -> Option<mpsc::Sender<ActivityDelta>> {
        self.tx.clone()
    }
}

impl Debug for ActivityEmitter {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("ActivityEmitter")
            .field("is_noop", &self.tx.is_none())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delta(activity_type: &str) -> ActivityDelta {
        ActivityDelta {
            activity_type: activity_type.to_string(),
            patch: vec![],
        }
    }

    #[tokio::test]
    async fn noop_emit_does_not_panic() {
        ActivityEmitter::noop()
            .emit(delta("test"))
            .await;
    }

    #[tokio::test]
    async fn emit_delivers_delta_to_receiver() {
        let (tx, mut rx) = mpsc::channel(4);
        let emitter = ActivityEmitter::new(tx);
        emitter.emit(delta("my_type")).await;
        let received = rx
            .recv()
            .await
            .expect("expected a delta");
        assert_eq!(received.activity_type, "my_type");
    }

    #[tokio::test]
    async fn emit_drops_delta_silently_when_channel_full() {
        let (tx, _rx) = mpsc::channel(1);
        let emitter = ActivityEmitter::new(tx);
        // Fill the channel, then overflow it. Neither call should panic.
        emitter.emit(delta("first")).await;
        emitter.emit(delta("overflow")).await;
    }
}
