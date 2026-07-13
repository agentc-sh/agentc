// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#![allow(deprecated)]

use async_trait::async_trait;
use futures::Stream;
use std::{
    pin::Pin,
    sync::LazyLock,
    task::{Context, Poll},
    time::Instant,
};

use agentc_telemetry::{
    Span, Instrument, field, info_span,
    metrics::{Histogram, KeyValue, meter},
    semconv::{attribute, metric},
};

use crate::{
    errors::ModelError,
    stream::ChatCompletionStream,
    types::{
        stream::CompletionStreamEvent,
        usage::TokenUsage,
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
        request::CompletionRequest,
    },
    traits::CompletionModel,
};

static OPERATION_DURATION: LazyLock<Histogram<f64>> = LazyLock::new(|| {
    meter("agentc-model")
        .f64_histogram(metric::GEN_AI_CLIENT_OPERATION_DURATION)
        .with_unit("s")
        .with_description("Duration of GenAI chat operations.")
        .build()
});

static TOKEN_USAGE: LazyLock<Histogram<u64>> = LazyLock::new(|| {
    meter("agentc-model")
        .u64_histogram(metric::GEN_AI_CLIENT_TOKEN_USAGE)
        .with_unit("{token}")
        .with_description("Number of tokens used in GenAI chat operations.")
        .build()
});

static TIME_TO_FIRST_CHUNK: LazyLock<Histogram<f64>> = LazyLock::new(|| {
    meter("agentc-model")
        .f64_histogram(metric::GEN_AI_CLIENT_OPERATION_TIME_TO_FIRST_CHUNK)
        .with_unit("s")
        .with_description("Time to the first streamed chunk of a GenAI chat operation.")
        .build()
});

static TIME_PER_OUTPUT_CHUNK: LazyLock<Histogram<f64>> = LazyLock::new(|| {
    meter("agentc-model")
        .f64_histogram(metric::GEN_AI_CLIENT_OPERATION_TIME_PER_OUTPUT_CHUNK)
        .with_unit("s")
        .with_description("Mean latency between streamed output chunks of a GenAI chat operation.")
        .build()
});

/// Wraps a provider [`ChatCompletionStream`] to record the GenAI `chat` span
/// attributes and `gen_ai.client.*` metrics as the stream is consumed. Transparent
/// to the consumer: it yields exactly the inner stream's events.
pub(crate) struct InstrumentedStream {
    inner: ChatCompletionStream,
    span: Span,
    attrs: Vec<KeyValue>,
    start: Instant,
    first_chunk: Option<f64>,
    output_chunks: u64,
    error_type: Option<&'static str>,
    saw_done: bool,
    finished: bool,
}

impl InstrumentedStream {
    pub(crate) fn new(
        inner: ChatCompletionStream,
        span: Span,
        attrs: Vec<KeyValue>,
        start: Instant,
    ) -> Self {
        Self {
            inner,
            span,
            attrs,
            start,
            first_chunk: None,
            output_chunks: 0,
            error_type: None,
            saw_done: false,
            finished: false,
        }
    }

    /// Records the operation-duration metric for a completion that failed before
    /// producing a stream. The caller records `error.type` on the span it owns.
    pub(crate) fn record_failed(attrs: &[KeyValue], start: Instant, error_type: &'static str) {
        let mut attrs = attrs.to_vec();
        attrs.push(KeyValue::new(attribute::ERROR_TYPE, error_type));

        OPERATION_DURATION.record(start.elapsed().as_secs_f64(), &attrs);
    }

    fn record_first_chunk(&mut self) {
        let elapsed = self.start.elapsed().as_secs_f64();
        self.first_chunk = Some(elapsed);

        TIME_TO_FIRST_CHUNK.record(elapsed, &self.attrs);
    }

    fn record_done(&mut self, usage: &TokenUsage) {
        self.saw_done = true;

        self.span
            .record("gen_ai.usage.input_tokens", usage.input_tokens as i64);
        self.span
            .record("gen_ai.usage.output_tokens", usage.output_tokens as i64);
        if let Some(cache_read) = usage.cache_input_tokens {
            self.span
                .record("gen_ai.usage.cache_read.input_tokens", cache_read as i64);
        }

        let mut input_attrs = self.attrs.clone();
        input_attrs.push(KeyValue::new(attribute::GEN_AI_TOKEN_TYPE, "input"));
        TOKEN_USAGE.record(usage.input_tokens as u64, &input_attrs);

        let mut output_attrs = self.attrs.clone();
        output_attrs.push(KeyValue::new(attribute::GEN_AI_TOKEN_TYPE, "output"));
        TOKEN_USAGE.record(usage.output_tokens as u64, &output_attrs);
    }

    fn finalize(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;

        let duration = self.start.elapsed().as_secs_f64();

        let mut attrs = self.attrs.clone();
        if let Some(error_type) = self.error_type {
            self.span.record("error.type", error_type);
            attrs.push(KeyValue::new(attribute::ERROR_TYPE, error_type));
        }
        OPERATION_DURATION.record(duration, &attrs);

        if let Some(first_chunk) = self.first_chunk {
            let intervals = self.output_chunks.saturating_sub(1).max(1) as f64;
            TIME_PER_OUTPUT_CHUNK.record((duration - first_chunk) / intervals, &self.attrs);
        }
    }
}

impl Stream for InstrumentedStream {
    type Item = Result<CompletionStreamEvent, ModelError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        let polled = {
            let _entered = this.span.enter();
            Pin::new(&mut this.inner).poll_next(cx)
        };

        match polled {
            Poll::Ready(Some(Ok(event))) => {
                if this.first_chunk.is_none() {
                    this.record_first_chunk();
                }

                match &event {
                    CompletionStreamEvent::TextDelta { .. }
                    | CompletionStreamEvent::ToolCallDelta { .. } => this.output_chunks += 1,
                    CompletionStreamEvent::Done(usage) => this.record_done(usage),
                    _ => {}
                }

                Poll::Ready(Some(Ok(event)))
            }
            Poll::Ready(Some(Err(error))) => {
                this.error_type = Some(error.error_type());

                Poll::Ready(Some(Err(error)))
            }
            Poll::Ready(None) => {
                this.finalize();

                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for InstrumentedStream {
    fn drop(&mut self) {
        if self.finished {
            return;
        }

        if self.error_type.is_none() && !self.saw_done {
            self.error_type = Some("cancelled");
        }

        self.finalize();
    }
}

pub struct InstrumentedCompletionModel<M> {
    inner: M,
}

impl<M> InstrumentedCompletionModel<M> {
    pub fn new(inner: M) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<M: CompletionModel + Send + Sync> CompletionModel for InstrumentedCompletionModel<M> {
    fn model(&self) -> &ModelId {
        self.inner.model()
    }

    fn otel_provider_name(&self) -> &'static str {
        self.inner.otel_provider_name()
    }

    fn provider(&self) -> ProviderId {
        self.inner.provider()
    }

    fn inference_params(&self) -> &InferenceParams {
        self.inner.inference_params()
    }

    async fn send(
        &self,
        request: CompletionRequest,
    ) -> Result<ChatCompletionStream, ModelError> {
        let start = Instant::now();
        let model = self.model().to_string();
        let span = info_span!(
            "chat",
            otel.name = %format!("chat {model}"),
            otel.kind = "client",
            gen_ai.operation.name = "chat",
            gen_ai.provider.name = self.otel_provider_name(),
            gen_ai.request.model = model.as_str(),
            gen_ai.usage.input_tokens = field::Empty,
            gen_ai.usage.output_tokens = field::Empty,
            gen_ai.usage.cache_read.input_tokens = field::Empty,
            error.type = field::Empty,
        );
        let attributes = vec![
            KeyValue::new(attribute::GEN_AI_OPERATION_NAME, "chat"),
            KeyValue::new(attribute::GEN_AI_PROVIDER_NAME, self.otel_provider_name()),
            KeyValue::new(attribute::GEN_AI_REQUEST_MODEL, model),
        ];

        match self.inner
            .send(request)
            .instrument(span.clone())
            .await
        {
            Ok(stream) => Ok(ChatCompletionStream::new(InstrumentedStream::new(
                stream,
                span,
                attributes,
                start,
            ))),
            Err(error) => {
                span.record(attribute::ERROR_TYPE, error.error_type());
                InstrumentedStream::record_failed(&attributes, start, error.error_type());
                Err(error)
            },
        }
    }
}

pub trait AsInstrumentedModel: Sized {
    fn as_instrumented(self) -> InstrumentedCompletionModel<Self>;
}

impl<M: CompletionModel + 'static> AsInstrumentedModel for M {
    fn as_instrumented(self) -> InstrumentedCompletionModel<Self> {
        InstrumentedCompletionModel::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures::{StreamExt, stream};

    #[tokio::test]
    async fn adapter_yields_the_inner_events_unchanged() {
        let inner = ChatCompletionStream::new(stream::iter(vec![
            Ok(CompletionStreamEvent::TextDelta { delta: "hi".to_string() }),
            Ok(CompletionStreamEvent::Done(TokenUsage {
                input_tokens: 3,
                output_tokens: 5,
                cache_input_tokens: None,
            })),
        ]));

        let collected = InstrumentedStream::new(inner, Span::none(), Vec::new(), Instant::now())
            .map(|event| event.expect("event"))
            .collect::<Vec<_>>()
            .await;

        assert!(matches!(collected.as_slice(), [
            CompletionStreamEvent::TextDelta { .. },
            CompletionStreamEvent::Done(_),
        ]));
    }
}
