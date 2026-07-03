// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

/// Counts tokens in a string. Implementations may use a real tokenizer or an approximation.
///
/// The counter is intentionally decoupled from any specific model or provider.
/// Callers are responsible for choosing an implementation that matches their target model.
pub trait TokenCounter: Send + Sync {
    fn count(&self, text: &str) -> usize;
}

/// A fast approximation that estimates token count as `text.len() / 4`.
///
/// Suitable for rough budget enforcement where accuracy is not critical.
/// For production use, prefer a real tokenizer such as [`TiktokenCounter`](agentc_prompt::counter::TiktokenCounter).
pub struct CharApproxCounter;

impl TokenCounter for CharApproxCounter {
    fn count(&self, text: &str) -> usize {
        (text.len() / 4).max(1)
    }
}

/// An accurate token counter backed by the tiktoken `o200k_base` encoding.
///
/// `o200k_base` is used by GPT-4o and the `o`-series models and is a good
/// default for any OpenAI-compatible provider.
///
/// Construction is cheap: the underlying BPE table is initialized once via a
/// process-global singleton and subsequent calls just borrow the static reference.
#[cfg(feature = "tiktoken")]
#[derive(Clone, Copy)]
pub struct TiktokenCounter {
    bpe: &'static tiktoken_rs::CoreBPE,
}

#[cfg(feature = "tiktoken")]
impl TiktokenCounter {
    /// Returns a counter using the `o200k_base` encoding (GPT-4o, o-series).
    pub fn o200k_base() -> Self {
        Self { bpe: tiktoken_rs::o200k_base_singleton() }
    }

    /// Returns a counter using the `cl100k_base` encoding (GPT-4, GPT-3.5).
    pub fn cl100k_base() -> Self {
        Self {
            bpe: tiktoken_rs::cl100k_base_singleton(),
        }
    }
}

#[cfg(feature = "tiktoken")]
impl TokenCounter for TiktokenCounter {
    fn count(&self, text: &str) -> usize {
        self.bpe
            .encode_with_special_tokens(text)
            .len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_approx_divides_length_by_four() {
        let c = CharApproxCounter;
        assert_eq!(c.count("1234"), 1); // exactly 4 chars
        assert_eq!(c.count("12345678"), 2); // exactly 8 chars
        assert_eq!(c.count("1234567890ab"), 3); // 12 chars
    }

    #[test]
    fn char_approx_minimum_is_one_for_short_input() {
        let c = CharApproxCounter;
        assert_eq!(c.count(""), 1); // 0 / 4 = 0, clamped to 1
        assert_eq!(c.count("abc"), 1); // 3 / 4 = 0, clamped to 1
    }

    // ---- TiktokenCounter ---------------------------------------------------

    #[cfg(feature = "tiktoken")]
    #[test]
    fn tiktoken_o200k_counts_known_token_sequence() {
        // "hello world" tokenizes to ["hello", " world"] = 2 tokens in o200k_base
        let c = TiktokenCounter::o200k_base();
        assert_eq!(c.count("hello world"), 2);
    }

    #[cfg(feature = "tiktoken")]
    #[test]
    fn tiktoken_cl100k_counts_known_token_sequence() {
        // "hello world" is also 2 tokens in cl100k_base
        let c = TiktokenCounter::cl100k_base();
        assert_eq!(c.count("hello world"), 2);
    }

    #[cfg(feature = "tiktoken")]
    #[test]
    fn tiktoken_counter_is_copy() {
        let c = TiktokenCounter::o200k_base();
        let c2 = c; // Copy, not move
        assert_eq!(c.count("test"), c2.count("test"));
    }
}
