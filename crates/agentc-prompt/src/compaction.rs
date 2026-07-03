// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;

use crate::{
    buffer::{TokenBudget, TrackedMessage},
    counter::TokenCounter,
};

/// An optional trait for messages that can be grouped together during compaction.
pub trait MessageGroup {
    fn group_id(&self) -> Option<String>;
}

impl MessageGroup for String {
    fn group_id(&self) -> Option<String> {
        None
    }
}

impl MessageGroup for &str {
    fn group_id(&self) -> Option<String> {
        None
    }
}

/// A pluggable strategy for reducing a buffer's non-pinned token count to within budget.
///
/// Implementations receive only the non-pinned messages. The buffer partitions
/// by pin status before calling `compact` and reconstructs the original ordering
/// afterward, so implementations never need to check or respect pin flags.
#[async_trait]
pub trait CompactionStrategy<T: Send>: Send + Sync {
    async fn compact(
        &self,
        messages: &mut Vec<TrackedMessage<T>>,
        budget: &TokenBudget,
        counter: &dyn TokenCounter,
    );
}

/// Drops non-pinned messages oldest-first until the buffer is within budget.
///
/// Messages that return a `group_id` from [`MessageGroup`] are treated as atomic
/// units: all messages sharing the same group ID are dropped together rather than
/// individually. This prevents orphaned tool result messages when an assistant
/// message with tool calls is evicted. Messages with no group ID are treated as
/// independent singletons and may be dropped on their own.
///
/// This strategy makes no attempt to summarize or truncate content; messages are
/// either kept whole or removed entirely.
pub struct TailWindow;

#[async_trait]
impl<T: Send + MessageGroup> CompactionStrategy<T> for TailWindow {
    async fn compact(
        &self,
        messages: &mut Vec<TrackedMessage<T>>,
        budget: &TokenBudget,
        _counter: &dyn TokenCounter,
    ) {
        let mut total: usize = messages
            .iter()
            .map(|m| m.token_count)
            .sum();
        let limit = budget.effective();

        if total <= limit {
            return;
        }

        // Build an ordered list of groups. Each entry is a list of indices into
        // `messages`. Named groups (group_id == Some) collect all their members
        // together; singletons (group_id == None) each occupy their own entry.
        // Group order is determined by the first appearance of each group ID.
        let mut groups = Vec::<Vec<usize>>::new();
        let mut group_map = HashMap::<String, usize>::new();

        for (idx, msg) in messages.iter().enumerate() {
            match msg.message.group_id() {
                Some(gid) => {
                    if let Some(&slot) = group_map.get(&gid) {
                        groups[slot].push(idx);
                    } else {
                        group_map.insert(gid, groups.len());
                        groups.push(vec![idx]);
                    }
                }
                None => groups.push(vec![idx]),
            }
        }

        // Drop the oldest complete groups until back within budget.
        let mut to_drop = HashSet::new();

        for group in &groups {
            if total <= limit {
                break;
            }

            let group_tokens = group
                .iter()
                .map(|&idx| messages[idx].token_count)
                .sum::<usize>();

            for &idx in group {
                to_drop.insert(idx);
            }

            total -= group_tokens;
        }

        // Retain only messages whose index is not marked for removal.
        let mut idx = 0;
        messages.retain(|_| {
            let keep = !to_drop.contains(&idx);
            idx += 1;
            keep
        });
    }
}

/// A no-op compaction strategy that always leaves the buffer unchanged.
///
/// Useful as a default or placeholder when you want to disable compaction
/// without changing call sites that accept a `&dyn CompactionStrategy<T>`.
pub struct NoCompaction;

#[async_trait]
impl<T: Send> CompactionStrategy<T> for NoCompaction {
    async fn compact(
        &self,
        _messages: &mut Vec<TrackedMessage<T>>,
        _budget: &TokenBudget,
        _counter: &dyn TokenCounter,
    ) {
    }
}
