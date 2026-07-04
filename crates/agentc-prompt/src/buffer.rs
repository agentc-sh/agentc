// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use crate::{
    compaction::CompactionStrategy,
    counter::{CharApproxCounter, TokenCounter},
};

/// Calculates the token count for a value using a provided counter.
pub trait TokenCount {
    fn token_count(&self, counter: &dyn TokenCounter) -> usize;
}

impl TokenCount for String {
    fn token_count(&self, counter: &dyn TokenCounter) -> usize {
        counter.count(self)
    }
}

impl TokenCount for &str {
    fn token_count(&self, counter: &dyn TokenCounter) -> usize {
        counter.count(self)
    }
}

/// A message held in a `MessageBuffer`, along with its token count, pin status,
/// and original insertion sequence number.
#[derive(Debug, Clone)]
pub struct TrackedMessage<T> {
    pub message: T,
    pub token_count: usize,
    pub pinned: bool,
    pub idx: usize,
}

/// Token budget constraints passed to a [`CompactionStrategy`](crate::compaction::CompactionStrategy).
#[derive(Debug, Clone)]
pub struct TokenBudget {
    /// Total tokens the model accepts as input (the context window size).
    pub input_limit: usize,
    /// Tokens to reserve for the model's output response.
    /// The effective compaction target is `input_limit - output_reserve`.
    pub output_reserve: usize,
    /// If set, a non-pinned message with fewer tokens than this floor must be
    /// dropped entirely rather than partially compacted. Strategies that only
    /// drop messages (such as `TailWindow`) apply this as a post-check: if
    /// dropping would leave a message smaller than the floor, drop it outright.
    pub min_message_tokens: Option<usize>,
}

impl TokenBudget {
    pub fn new(input_limit: usize, output_reserve: usize) -> Self {
        Self {
            input_limit,
            output_reserve,
            min_message_tokens: None,
        }
    }

    pub fn adjusted(&self, reserved_tokens: usize, output_reserve: usize) -> Self {
        Self {
            input_limit: self
                .effective()
                .saturating_sub(reserved_tokens),
            output_reserve,
            min_message_tokens: self.min_message_tokens,
        }
    }

    pub fn with_min_message_tokens(mut self, min: usize) -> Self {
        self.min_message_tokens = Some(min);
        self
    }

    pub fn effective(&self) -> usize {
        self.input_limit
            .saturating_sub(self.output_reserve)
    }
}

/// An ephemeral, ordered buffer of tracked messages with a token budget.
///
/// The buffer is intended to be constructed fresh for each model call from the
/// persisted conversation state. Compaction operates on the in-memory buffer
/// only and does not affect stored data.
///
/// Token counts are computed automatically at push time using the buffer's
/// own `TokenCounter`. The default counter is `CharApproxCounter`. For
/// production accuracy, construct with a tiktoken-backed counter.
///
/// Pinned messages (typically rendered prompt messages) are never passed to a
/// compaction strategy. The buffer partitions by pin status before compaction
/// and reassembles in the original insertion order afterward.
pub struct MessageBuffer<T: TokenCount> {
    messages: Vec<TrackedMessage<T>>,
    budget: TokenBudget,
    counter: Arc<dyn TokenCounter>,
    next_idx: usize,
}

impl<T: TokenCount> MessageBuffer<T> {
    pub fn new(budget: TokenBudget) -> Self {
        Self {
            messages: Vec::new(),
            budget,
            counter: Arc::new(CharApproxCounter),
            next_idx: 0,
        }
    }

    pub fn builder() -> MessageBufferBuilder {
        MessageBufferBuilder::new()
    }

    pub fn use_counter<C>(&mut self, counter: C)
    where
        C: TokenCounter + 'static,
    {
        self.counter = Arc::new(counter);
    }

    pub fn use_counter_arc(&mut self, counter: Arc<dyn TokenCounter>) {
        self.counter = counter;
    }

    fn next_idx(&mut self) -> usize {
        let idx = self.next_idx;
        self.next_idx += 1;
        idx
    }

    /// Push a compactable message. Token count is computed automatically.
    pub fn push(&mut self, message: T) {
        let token_count = message.token_count(self.counter.as_ref());
        let idx = self.next_idx();

        self.messages
            .push(TrackedMessage { message, token_count, pinned: false, idx });
    }

    /// Push a pinned message that no compaction strategy will remove or modify.
    /// Token count is computed automatically.
    pub fn push_pinned(&mut self, message: T) {
        let token_count = message.token_count(self.counter.as_ref());
        let idx = self.next_idx();

        self.messages
            .push(TrackedMessage { message, token_count, pinned: true, idx });
    }

    /// Push all items from an iterator as compactable messages.
    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for message in iter {
            self.push(message);
        }
    }

    /// Push all items from an iterator as pinned messages.
    pub fn extend_pinned<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for message in iter {
            self.push_pinned(message);
        }
    }

    pub fn total_tokens(&self) -> usize {
        self.messages
            .iter()
            .map(|m| m.token_count)
            .sum()
    }

    pub fn is_over_budget(&self) -> bool {
        self.total_tokens() > self.budget.effective()
    }

    pub fn pinned_tokens(&self) -> usize {
        self.messages
            .iter()
            .filter(|m| m.pinned)
            .map(|m| m.token_count)
            .sum()
    }

    pub fn compactable_budget(&self) -> usize {
        self.budget
            .effective()
            .saturating_sub(self.pinned_tokens())
    }

    pub fn budget(&self) -> &TokenBudget {
        &self.budget
    }

    /// Apply a compaction strategy. No-op if the buffer is already within budget.
    ///
    /// Pinned messages are partitioned out before the strategy is called and
    /// reinserted afterward in the original insertion order, so strategies
    /// never see or touch pinned messages.
    pub async fn compact_with(&mut self, strategy: &dyn CompactionStrategy<T>)
    where
        T: Send,
    {
        if !self.is_over_budget() {
            return;
        }

        // Separate pinned and non-pinned, preserving idx on both sides.
        let (mut pinned, mut compactable) = self
            .messages
            .drain(..)
            .partition::<Vec<_>, _>(|m| m.pinned);

        strategy
            .compact(
                &mut compactable,
                // Build an adjusted budget that reflects only the capacity left for
                // non-pinned messages after pinned tokens have been accounted for.
                &self.budget.adjusted(
                    pinned
                        .iter()
                        .map(|m| m.token_count)
                        .sum(),
                    0,
                ),
                self.counter.as_ref(),
            )
            .await;

        // Merge back in original insertion order using the index.
        pinned.extend(compactable);
        pinned.sort_unstable_by_key(|m| m.idx);

        self.messages = pinned;
    }

    pub fn messages(&self) -> impl Iterator<Item = &T> {
        self.messages.iter().map(|m| &m.message)
    }

    pub fn into_messages(self) -> impl Iterator<Item = T> {
        self.messages
            .into_iter()
            .map(|m| m.message)
    }

    pub fn tracked(&self) -> &[TrackedMessage<T>] {
        &self.messages
    }
}

pub struct MessageBufferBuilder {
    budget: Option<TokenBudget>,
    counter: Option<Arc<dyn TokenCounter>>,
}

impl Default for MessageBufferBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageBufferBuilder {
    pub fn new() -> Self {
        Self { budget: None, counter: None }
    }

    pub fn with_budget(mut self, budget: TokenBudget) -> Self {
        self.budget = Some(budget);
        self
    }

    pub fn with_counter<C>(mut self, counter: C) -> Self
    where
        C: TokenCounter + 'static,
    {
        self.counter = Some(Arc::new(counter));
        self
    }

    pub fn with_counter_arc(mut self, counter: Arc<dyn TokenCounter>) -> Self {
        self.counter = Some(counter);
        self
    }

    pub fn build<T: TokenCount>(self) -> MessageBuffer<T> {
        let mut buffer = MessageBuffer::new(
            self.budget
                .expect("TokenBudget is required"),
        );

        if let Some(counter) = self.counter {
            buffer.use_counter_arc(counter);
        }

        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compaction::{NoCompaction, TailWindow, MessageGroup};

    fn budget(input: usize, output: usize) -> TokenBudget {
        TokenBudget::new(input, output)
    }

    // A minimal test message type.
    #[derive(Debug, Clone, PartialEq)]
    struct Msg(&'static str);

    impl MessageGroup for Msg {
        fn group_id(&self) -> Option<String> {
            None
        }
    }

    impl TokenCount for Msg {
        fn token_count(&self, counter: &dyn TokenCounter) -> usize {
            counter.count(self.0)
        }
    }

    // A message type that has no countable content.
    #[derive(Debug, Clone)]
    struct EmptyMsg;

    impl TokenCount for EmptyMsg {
        fn token_count(&self, _counter: &dyn TokenCounter) -> usize {
            0
        }
    }

    #[derive(Debug, Clone)]
    struct CompoundMsg(&'static [&'static str]);

    impl TokenCount for CompoundMsg {
        fn token_count(&self, counter: &dyn TokenCounter) -> usize {
            self.0
                .iter()
                .map(|content| counter.count(content))
                .sum()
        }
    }

    // ---- TokenBudget -------------------------------------------------------

    #[test]
    fn budget_effective_subtracts_output_reserve() {
        assert_eq!(budget(200, 50).effective(), 150);
    }

    #[test]
    fn budget_effective_saturates_at_zero_when_reserve_exceeds_limit() {
        assert_eq!(budget(10, 20).effective(), 0);
    }

    #[test]
    fn budget_min_message_tokens_is_none_by_default() {
        assert!(
            budget(100, 0)
                .min_message_tokens
                .is_none()
        );
    }

    #[test]
    fn budget_with_min_message_tokens_sets_field() {
        let b = budget(100, 0).with_min_message_tokens(8);
        assert_eq!(b.min_message_tokens, Some(8));
    }

    // ---- TokenCount --------------------------------------------------------

    #[test]
    fn empty_message_counts_as_zero_tokens() {
        let mut buf = MessageBuffer::new(budget(100, 0));
        buf.push(EmptyMsg);
        assert_eq!(buf.total_tokens(), 0);
    }

    #[test]
    fn compound_message_sums_content_without_joining() {
        let mut buf = MessageBuffer::new(budget(100, 0));
        buf.push(CompoundMsg(&["aaaa", "bbbbbbbb"]));
        assert_eq!(buf.total_tokens(), 3);
    }

    // ---- MessageBuffer push / query ----------------------------------------

    #[test]
    fn push_creates_unpinned_entry() {
        let mut buf = MessageBuffer::new(budget(100, 0));
        buf.push(Msg("hello"));
        assert!(!buf.tracked()[0].pinned);
    }

    #[test]
    fn push_pinned_creates_pinned_entry() {
        let mut buf = MessageBuffer::new(budget(100, 0));
        buf.push_pinned(Msg("hello"));
        assert!(buf.tracked()[0].pinned);
    }

    #[test]
    fn push_counts_tokens_automatically() {
        // "hello world" is 11 chars, 11/4 = 2 via CharApproxCounter
        let mut buf = MessageBuffer::new(budget(100, 0));
        buf.push(Msg("hello world"));
        assert_eq!(buf.tracked()[0].token_count, 2);
    }

    #[test]
    fn total_tokens_sums_all_entries() {
        let mut buf = MessageBuffer::new(budget(100, 0));
        buf.push(Msg("aaaa")); // 4 chars -> 1 token
        buf.push_pinned(Msg("bbbbbbbb")); // 8 chars -> 2 tokens
        assert_eq!(buf.total_tokens(), 3);
    }

    #[test]
    fn is_over_budget_uses_effective_not_input_limit() {
        // effective = 5, message = "aaaaaaa" = 7 chars = 1 token... hmm.
        // Use a longer string to reliably be over budget.
        let mut buf = MessageBuffer::new(budget(10, 5)); // effective = 5
        // "aaaaaaaaaaaaaaaaaaaaaaaaa" = 25 chars = 6 tokens
        buf.push(Msg("aaaaaaaaaaaaaaaaaaaaaaaaa"));
        assert!(buf.is_over_budget());
    }

    #[test]
    fn is_not_over_budget_when_equal_to_effective_limit() {
        let mut buf = MessageBuffer::new(budget(20, 0)); // effective = 20
        buf.push(Msg("aaaaaaaaaaaaaaaaaaaaaaaaa")); // 25 chars = 6 tokens, well under 20
        assert!(!buf.is_over_budget());
    }

    #[test]
    fn messages_iter_yields_values_in_insertion_order() {
        let mut buf = MessageBuffer::new(budget(100, 0));
        buf.push(Msg("first"));
        buf.push(Msg("second"));
        buf.push(Msg("third"));
        let msgs: Vec<&Msg> = buf.messages().collect();
        assert_eq!(msgs[0].0, "first");
        assert_eq!(msgs[1].0, "second");
        assert_eq!(msgs[2].0, "third");
    }

    #[test]
    fn into_messages_consumes_buffer_and_yields_values() {
        let mut buf = MessageBuffer::new(budget(100, 0));
        buf.push(Msg("a"));
        buf.push(Msg("b"));
        let msgs: Vec<Msg> = buf.into_messages().collect();
        assert_eq!(msgs, vec![Msg("a"), Msg("b")]);
    }

    // ---- Sequence ordering -------------------------------------------------

    #[tokio::test]
    async fn compaction_preserves_interleaved_insertion_order() {
        // interleave pinned and non-pinned: seq 0=pinned, 1=non, 2=pinned, 3=non
        // Pinned tokens: "p0"=1, "p2"=1 -> 2 pinned tokens.
        // effective budget = 4, compactable budget = 4 - 2 = 2.
        // "drop-me______" = 13 chars = 3 tokens, "keep_______" = 11 chars = 2 tokens.
        // Total compactable = 5, over budget of 2, so "drop-me______" is dropped first.
        // After: remaining order should be seq 0(p0), 2(p2), 3(keep).
        let mut buf = MessageBuffer::new(budget(4, 0));
        buf.push_pinned(Msg("p0")); // seq 0
        buf.push(Msg("drop-me______")); // seq 1, 3 tokens
        buf.push_pinned(Msg("p2")); // seq 2
        buf.push(Msg("keep_______")); // seq 3, 2 tokens

        buf.compact_with(&TailWindow).await;

        let result: Vec<&str> = buf.messages().map(|m| m.0).collect();
        assert_eq!(result, vec!["p0", "p2", "keep_______"]);
    }

    // ---- TailWindow compaction ---------------------------------------------

    #[tokio::test]
    async fn tail_window_drops_oldest_non_pinned_first() {
        let mut buf = MessageBuffer::new(budget(10, 0));
        // Each of these is 16 chars = 4 tokens
        buf.push(Msg("aaaaaaaaaaaaaaaa"));
        buf.push(Msg("bbbbbbbbbbbbbbbb"));
        buf.push(Msg("cccccccccccccccc"));

        buf.compact_with(&TailWindow).await;

        let remaining: Vec<&str> = buf.messages().map(|m| m.0).collect();
        assert!(!remaining.contains(&"aaaaaaaaaaaaaaaa"), "oldest should be dropped");
        assert!(remaining.contains(&"bbbbbbbbbbbbbbbb"));
        assert!(remaining.contains(&"cccccccccccccccc"));
    }

    #[tokio::test]
    async fn tail_window_preserves_pinned_messages() {
        let mut buf = MessageBuffer::new(budget(5, 0));
        // pinned messages well over budget on their own
        buf.push_pinned(Msg("pppppppppppppppp")); // 4 tokens
        buf.push_pinned(Msg("qqqqqqqqqqqqqqqq")); // 4 tokens
        buf.push(Msg("uuuuuuuuuuuuuuuu")); // 4 tokens

        buf.compact_with(&TailWindow).await;

        let remaining: Vec<&str> = buf.messages().map(|m| m.0).collect();
        assert!(remaining.contains(&"pppppppppppppppp"));
        assert!(remaining.contains(&"qqqqqqqqqqqqqqqq"));
        assert!(!remaining.contains(&"uuuuuuuuuuuuuuuu"));
    }

    #[tokio::test]
    async fn compact_with_is_noop_when_already_within_budget() {
        let mut buf = MessageBuffer::new(budget(100, 0));
        buf.push(Msg("a"));
        buf.push(Msg("b"));

        buf.compact_with(&TailWindow).await;

        assert_eq!(buf.messages().count(), 2);
    }

    #[tokio::test]
    async fn compact_with_handles_empty_buffer() {
        let mut buf: MessageBuffer<Msg> = MessageBuffer::new(budget(10, 0));
        buf.compact_with(&TailWindow).await;
        assert_eq!(buf.messages().count(), 0);
    }

    // ---- NoCompaction -------------------------------------------------------

    #[tokio::test]
    async fn no_compaction_leaves_buffer_unchanged_when_over_budget() {
        let mut buf = MessageBuffer::new(budget(5, 0));
        buf.push(Msg("aaaaaaaaaaaaaaaa")); // 4 tokens
        buf.push(Msg("bbbbbbbbbbbbbbbb")); // 4 tokens

        buf.compact_with(&NoCompaction).await;

        assert_eq!(buf.messages().count(), 2, "NoCompaction must not remove messages");
    }
}
