// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#![allow(deprecated)]

use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::{
    pin::Pin,
    sync::LazyLock,
    task::{Context, Poll},
    time::Instant,
};

use agentc_telemetry::{
    Instrument, Span, field, info_span,
    metrics::{Histogram, KeyValue, meter},
    semconv::{
        attribute,
        genai::{
            GenAiInputMessages, GenAiMessage, GenAiModality, GenAiOutputMessage,
            GenAiOutputMessages, GenAiPart, GenAiSystemInstructions, ToGenAiType,
        },
        metric,
    },
};

use crate::{
    errors::ModelError,
    stream::ChatCompletionStream,
    traits::CompletionModel,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
        media::{Audio, Document, Image, MediaData, Video},
        message::{
            AssistantContent, AssistantMessage, ChatMessage, SystemMessage, UserContent,
            UserMessage,
        },
        reasoning::{Reasoning, ReasoningContent},
        request::CompletionRequest,
        stream::{CompletionStreamEvent, CompletionStreamFinal},
        tools::{ToolCall, ToolResult, ToolResultContent},
    },
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

impl ToGenAiType for SystemMessage {
    type GenAiType = GenAiMessage;

    fn to_gen_ai_type(
        &self,
    ) -> Result<Self::GenAiType, serde_json::Error> {
        Ok(
            GenAiMessage::system([GenAiPart::text(self.content.clone())]),
        )
    }
}

impl ToGenAiType for UserMessage {
    type GenAiType = Vec<GenAiMessage>;

    fn to_gen_ai_type(
        &self,
    ) -> Result<Self::GenAiType, serde_json::Error> {
        let mut messages = Vec::new();
        let mut parts = Vec::new();

        for content in &self.content {
            match content {
                UserContent::Text(content) => {
                    parts.push(GenAiPart::text(content.clone()));
                }
                UserContent::ToolResult(result) => {
                    if !parts.is_empty() {
                        messages.push(GenAiMessage::user(std::mem::take(&mut parts)));
                    }

                    messages.push(GenAiMessage::tool([result.to_gen_ai_type()?]));
                }
                UserContent::Image(image) => {
                    parts.push(image.to_gen_ai_type()?);
                }
                UserContent::Audio(audio) => {
                    parts.push(audio.to_gen_ai_type()?);
                }
                UserContent::Video(video) => {
                    parts.push(video.to_gen_ai_type()?);
                }
                UserContent::Document(document) => {
                    parts.push(document.to_gen_ai_type()?);
                }
            }
        }

        if !parts.is_empty() {
            messages.push(GenAiMessage::user(parts));
        }

        Ok(messages)
    }
}

impl ToGenAiType for AssistantMessage {
    type GenAiType = GenAiMessage;

    fn to_gen_ai_type(
        &self,
    ) -> Result<Self::GenAiType, serde_json::Error> {
        let mut parts = Vec::new();

        for content in &self.content {
            match content {
                AssistantContent::Text(content) => {
                    parts.push(GenAiPart::text(content.clone()));
                }
                AssistantContent::ToolCall(call) => {
                    parts.push(call.to_gen_ai_type()?);
                }
                AssistantContent::Reasoning(reasoning) => {
                    parts.extend(reasoning.to_gen_ai_type()?);
                }
                AssistantContent::Image(image) => {
                    parts.push(image.to_gen_ai_type()?);
                }
            }
        }

        Ok(GenAiMessage::assistant(parts))
    }
}

impl ToGenAiType for ChatMessage {
    type GenAiType = Vec<GenAiMessage>;

    fn to_gen_ai_type(
        &self,
    ) -> Result<Self::GenAiType, serde_json::Error> {
        match self {
            ChatMessage::System(message) => Ok(vec![message.to_gen_ai_type()?]),
            ChatMessage::User(message) => message.to_gen_ai_type(),
            ChatMessage::Assistant(message) => Ok(vec![message.to_gen_ai_type()?]),
        }
    }
}

impl ToGenAiType for ToolCall {
    type GenAiType = GenAiPart;

    fn to_gen_ai_type(
        &self,
    ) -> Result<Self::GenAiType, serde_json::Error> {
        Ok(
            GenAiPart::tool_call(
                self.id.clone(),
                self.name.clone(),
                self.arguments.clone(),
            ),
        )
    }
}

impl ToGenAiType for ToolResult {
    type GenAiType = GenAiPart;

    fn to_gen_ai_type(
        &self,
    ) -> Result<Self::GenAiType, serde_json::Error> {
        Ok(
            GenAiPart::tool_call_response(
                self.call_id.clone(),
                match self.content.as_slice() {
                    [] => Value::Array(Vec::new()),
                    [ToolResultContent::Text(content)] => Value::String(content.clone()),
                    [ToolResultContent::Image(image)] => {
                        serde_json::to_value(image.to_gen_ai_type()?)?
                    }
                    contents => Value::Array(
                        contents
                            .iter()
                            .map(|content| match content {
                                ToolResultContent::Text(content) => {
                                    Ok(Value::String(content.clone()))
                                }
                                ToolResultContent::Image(image) => {
                                    serde_json::to_value(image.to_gen_ai_type()?)
                                }
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                    ),
                },
            ),
        )
    }
}

impl ToGenAiType for Reasoning {
    type GenAiType = Vec<GenAiPart>;

    fn to_gen_ai_type(
        &self,
    ) -> Result<Self::GenAiType, serde_json::Error> {
        Ok(
            self.gen_ai_content()
                .into_iter()
                .map(GenAiPart::reasoning)
                .collect(),
        )
    }
}

impl Reasoning {
    fn gen_ai_content(&self) -> Vec<String> {
        self.content
            .iter()
            .filter_map(|content| match content {
                ReasoningContent::Text { text, .. } => Some(text.clone()),
                ReasoningContent::Summary(summary) => Some(summary.clone()),
                ReasoningContent::Encrypted(_) | ReasoningContent::Redacted(_) => None,
            })
            .collect()
    }
}

impl ToGenAiType for Image {
    type GenAiType = GenAiPart;

    fn to_gen_ai_type(
        &self,
    ) -> Result<Self::GenAiType, serde_json::Error> {
        Ok(
            match &self.data {
                MediaData::Base64(content) => GenAiPart::blob(
                    self.media_type.clone(),
                    GenAiModality::Image,
                    content.clone(),
                ),
                MediaData::Url(uri) => GenAiPart::uri(
                    self.media_type.clone(),
                    GenAiModality::Image,
                    uri.to_string(),
                ),
            },
        )
    }
}

impl ToGenAiType for Audio {
    type GenAiType = GenAiPart;

    fn to_gen_ai_type(
        &self,
    ) -> Result<Self::GenAiType, serde_json::Error> {
        Ok(
            match &self.data {
                MediaData::Base64(content) => GenAiPart::blob(
                    self.media_type.clone(),
                    GenAiModality::Audio,
                    content.clone(),
                ),
                MediaData::Url(uri) => GenAiPart::uri(
                    self.media_type.clone(),
                    GenAiModality::Audio,
                    uri.to_string(),
                ),
            },
        )
    }
}

impl ToGenAiType for Video {
    type GenAiType = GenAiPart;

    fn to_gen_ai_type(
        &self,
    ) -> Result<Self::GenAiType, serde_json::Error> {
        Ok(
            match &self.data {
                MediaData::Base64(content) => GenAiPart::blob(
                    self.media_type.clone(),
                    GenAiModality::Video,
                    content.clone(),
                ),
                MediaData::Url(uri) => GenAiPart::uri(
                    self.media_type.clone(),
                    GenAiModality::Video,
                    uri.to_string(),
                ),
            },
        )
    }
}

impl ToGenAiType for Document {
    type GenAiType = GenAiPart;

    fn to_gen_ai_type(
        &self,
    ) -> Result<Self::GenAiType, serde_json::Error> {
        Ok(
            match &self.data {
                MediaData::Base64(content) => GenAiPart::blob(
                    self.media_type.clone(),
                    GenAiModality::Document,
                    content.clone(),
                ),
                MediaData::Url(uri) => GenAiPart::uri(
                    self.media_type.clone(),
                    GenAiModality::Document,
                    uri.to_string(),
                ),
            },
        )
    }
}

#[derive(Default)]
struct GenAiOutput {
    parts: Vec<GenAiOutputPart>,
}

impl GenAiOutput {
    fn observe(&mut self, event: &CompletionStreamEvent) {
        match event {
            CompletionStreamEvent::TextDelta { delta } => {
                if let Some(GenAiOutputPart::Text(content)) = self.parts.last_mut() {
                    content.push_str(delta);
                } else {
                    self.parts
                        .push(GenAiOutputPart::Text(delta.clone()));
                }
            }
            CompletionStreamEvent::ReasoningDelta { id, delta } => {
                if let Some(GenAiOutputPart::Reasoning { content, .. }) = self
                    .parts
                    .iter_mut()
                    .rev()
                    .find(|part| {
                        matches!(
                            part,
                            GenAiOutputPart::Reasoning {
                                id: part_id,
                                complete: false,
                                ..
                            } if part_id.as_ref() == id.as_ref()
                        )
                    })
                {
                    if let Some(content) = content.last_mut() {
                        content.push_str(delta);
                    } else {
                        content.push(delta.clone());
                    }
                } else {
                    self.parts
                        .push(GenAiOutputPart::Reasoning {
                            id: id.clone(),
                            content: vec![delta.clone()],
                            complete: false,
                        });
                }
            }
            CompletionStreamEvent::Reasoning(reasoning) => {
                if let Some(part) = self
                    .parts
                    .iter_mut()
                    .rev()
                    .find(|part| {
                        matches!(
                            part,
                            GenAiOutputPart::Reasoning {
                                id,
                                complete: false,
                                ..
                            } if id.as_ref() == reasoning.id.as_ref()
                        )
                    })
                {
                    *part = GenAiOutputPart::Reasoning {
                        id: reasoning.id.clone(),
                        content: reasoning.gen_ai_content(),
                        complete: true,
                    };
                } else {
                    self.parts
                        .push(GenAiOutputPart::Reasoning {
                            id: reasoning.id.clone(),
                            content: reasoning.gen_ai_content(),
                            complete: true,
                        });
                }
            }
            CompletionStreamEvent::ToolCall(call) => {
                self.parts
                    .push(GenAiOutputPart::ToolCall(call.clone()));
            }
            CompletionStreamEvent::ToolCallDelta { .. } | CompletionStreamEvent::Done(_) => {}
        }
    }

    fn to_json(&self, final_response: &CompletionStreamFinal) -> Result<String, serde_json::Error> {
        GenAiOutputMessages::new([GenAiOutputMessage::new(
            GenAiMessage::assistant(self.message_parts()),
            final_response
                .finish_reason
                .clone()
                .unwrap_or_else(|| {
                    if self
                        .parts
                        .iter()
                        .any(|part| matches!(part, GenAiOutputPart::ToolCall(_)))
                    {
                        "tool_call".to_string()
                    } else {
                        "unknown".to_string()
                    }
                }),
        )])
        .to_json()
    }

    fn message_parts(&self) -> Vec<GenAiPart> {
        let mut parts = Vec::new();

        for part in &self.parts {
            match part {
                GenAiOutputPart::Text(content) if !content.is_empty() => {
                    parts.push(GenAiPart::text(content.clone()));
                }
                GenAiOutputPart::Reasoning { content, .. } => {
                    parts.extend(
                        content
                            .iter()
                            .filter(|content| !content.is_empty())
                            .map(|content| GenAiPart::reasoning(content.clone())),
                    );
                }
                GenAiOutputPart::ToolCall(call) => {
                    parts.push(GenAiPart::tool_call(
                        call.id.clone(),
                        call.name.clone(),
                        call.arguments.clone(),
                    ));
                }
                _ => {}
            }
        }

        parts
    }
}

enum GenAiOutputPart {
    Text(String),
    Reasoning {
        id: Option<String>,
        content: Vec<String>,
        complete: bool,
    },
    ToolCall(ToolCall),
}

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
    output: GenAiOutput,
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
            output: GenAiOutput::default(),
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

    fn record_done(&mut self, final_response: &CompletionStreamFinal) {
        self.saw_done = true;

        if let Ok(output) = self.output.to_json(final_response) {
            self.span
                .record(attribute::GEN_AI_OUTPUT_MESSAGES, output.as_str());
        }

        self.span
            .record("gen_ai.usage.input_tokens", final_response.usage.input_tokens as i64);
        self.span
            .record("gen_ai.usage.output_tokens", final_response.usage.output_tokens as i64);
        if let Some(cache_read) = final_response.usage.cache_input_tokens {
            self.span
                .record("gen_ai.usage.cache_read.input_tokens", cache_read as i64);
        }

        let mut input_attrs = self.attrs.clone();
        input_attrs.push(KeyValue::new(attribute::GEN_AI_TOKEN_TYPE, "input"));
        TOKEN_USAGE.record(final_response.usage.input_tokens as u64, &input_attrs);

        let mut output_attrs = self.attrs.clone();
        output_attrs.push(KeyValue::new(attribute::GEN_AI_TOKEN_TYPE, "output"));
        TOKEN_USAGE.record(final_response.usage.output_tokens as u64, &output_attrs);
    }

    fn finalize(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;

        let duration = self.start.elapsed().as_secs_f64();

        let mut attrs = self.attrs.clone();
        if let Some(error_type) = self.error_type {
            self.span
                .record("error.type", error_type);
            attrs.push(KeyValue::new(attribute::ERROR_TYPE, error_type));
        }
        OPERATION_DURATION.record(duration, &attrs);

        if let Some(first_chunk) = self.first_chunk {
            let intervals = self
                .output_chunks
                .saturating_sub(1)
                .max(1) as f64;
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

                this.output.observe(&event);

                match &event {
                    CompletionStreamEvent::TextDelta { .. }
                    | CompletionStreamEvent::ToolCallDelta { .. } => this.output_chunks += 1,
                    CompletionStreamEvent::Done(final_response) => this.record_done(final_response),
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

    async fn send(&self, request: CompletionRequest) -> Result<ChatCompletionStream, ModelError> {
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
            gen_ai.system_instructions = field::Empty,
            gen_ai.input.messages = field::Empty,
            gen_ai.output.messages = field::Empty,
            error.type = field::Empty,
        );
        let attributes = vec![
            KeyValue::new(attribute::GEN_AI_OPERATION_NAME, "chat"),
            KeyValue::new(attribute::GEN_AI_PROVIDER_NAME, self.otel_provider_name()),
            KeyValue::new(attribute::GEN_AI_REQUEST_MODEL, model),
        ];

        let (system, latest, rest) = match request.messages.split_ref() {
            Ok(messages) => messages,
            Err(error) => {
                span.record(attribute::ERROR_TYPE, error.error_type());
                InstrumentedStream::record_failed(&attributes, start, error.error_type());

                return Err(error);
            }
        };

        if let Some(system) = system.as_ref()
            && let Ok(content) = GenAiSystemInstructions::new([
                GenAiPart::text(system.as_str()),
            ])
            .to_json()
        {
            span.record(attribute::GEN_AI_SYSTEM_INSTRUCTIONS, content.as_str());
        }

        if let Ok(content) = rest
            .iter()
            .map(|message| message.to_gen_ai_type())
            .chain([latest.to_gen_ai_type()])
            .collect::<Result<Vec<_>, _>>()
            .map(|messages| GenAiInputMessages::new(messages.into_iter().flatten()))
            .and_then(|messages| messages.to_json())
        {
            span.record(attribute::GEN_AI_INPUT_MESSAGES, content.as_str());
        }

        match self
            .inner
            .send(request)
            .instrument(span.clone())
            .await
        {
            Ok(stream) => Ok(ChatCompletionStream::new(InstrumentedStream::new(
                stream, span, attributes, start,
            ))),
            Err(error) => {
                span.record(attribute::ERROR_TYPE, error.error_type());
                InstrumentedStream::record_failed(&attributes, start, error.error_type());
                Err(error)
            }
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
    use futures::{StreamExt, stream};
    use serde_json::{Value, from_str, json};
    use std::time::Instant;
    use url::Url;

    use agentc_telemetry::{
        Span,
        semconv::genai::{
            GenAiInputMessages, GenAiPart, GenAiSystemInstructions, ToGenAiType,
        },
    };

    use crate::{
        instrument::{GenAiOutput, InstrumentedStream},
        stream::ChatCompletionStream,
        types::{
            media::{Audio, Image, MediaData},
            message::{
                AssistantContent, AssistantMessage, ChatHistory, ChatMessage, UserContent,
                UserMessage,
            },
            reasoning::{Reasoning, ReasoningContent},
            stream::{CompletionStreamEvent, CompletionStreamFinal, ToolCallDelta},
            tools::{ToolCall, ToolResult, ToolResultContent},
            usage::TokenUsage,
        },
    };

    #[test]
    fn input_messages_preserve_user_and_tool_result_order() {
        assert_eq!(
            from_str::<Value>(
                &GenAiInputMessages::new(
                    [
                        UserMessage {
                            content: vec![
                                UserContent::Text("before".to_string()),
                                UserContent::ToolResult(ToolResult {
                                    call_id: "call-1".to_string(),
                                    content: vec![ToolResultContent::Text("result".to_string())],
                                }),
                                UserContent::Text("after".to_string()),
                            ],
                        }
                        .to_gen_ai_type()
                        .expect("history should convert"),
                        UserMessage {
                            content: vec![UserContent::Text("latest".to_string())],
                        }
                        .to_gen_ai_type()
                        .expect("latest message should convert"),
                    ]
                    .into_iter()
                    .flatten(),
                )
                .to_json()
                .expect("input messages should serialize"),
            )
            .expect("input JSON should parse"),
            json!([
                {
                    "role": "user",
                    "parts": [
                        {
                            "type": "text",
                            "content": "before"
                        }
                    ]
                },
                {
                    "role": "tool",
                    "parts": [
                        {
                            "type": "tool_call_response",
                            "id": "call-1",
                            "response": "result"
                        }
                    ]
                },
                {
                    "role": "user",
                    "parts": [
                        {
                            "type": "text",
                            "content": "after"
                        }
                    ]
                },
                {
                    "role": "user",
                    "parts": [
                        {
                            "type": "text",
                            "content": "latest"
                        }
                    ]
                }
            ]),
        );
    }

    #[test]
    fn system_instructions_are_separate_from_input_messages() {
        let history = ChatHistory::new(vec![
            ChatMessage::system("first"),
            ChatMessage::system("second"),
            ChatMessage::user("question"),
        ]);
        let (system, latest, rest) = history
            .split_ref()
            .expect("history should split");

        assert_eq!(
            from_str::<Value>(
                &GenAiSystemInstructions::new([GenAiPart::text(
                    system
                        .as_deref()
                        .expect("system instructions should exist"),
                )])
                .to_json()
                .expect("system instructions should serialize"),
            )
            .expect("system instruction JSON should parse"),
            json!([
                {
                    "type": "text",
                    "content": "first\nsecond"
                }
            ]),
        );
        assert_eq!(
            from_str::<Value>(
                &GenAiInputMessages::new(
                    rest.iter()
                        .map(|message| {
                            message
                                .to_gen_ai_type()
                                .expect("history should convert")
                        })
                        .chain([
                            latest
                                .to_gen_ai_type()
                                .expect("latest message should convert"),
                        ])
                        .flatten(),
                )
                .to_json()
                .expect("input messages should serialize"),
            )
            .expect("input JSON should parse"),
            json!([
                {
                    "role": "user",
                    "parts": [
                        {
                            "type": "text",
                            "content": "question"
                        }
                    ]
                }
            ]),
        );
    }

    #[test]
    fn input_messages_preserve_url_and_base64_media() {
        assert_eq!(
            from_str::<Value>(
                &GenAiInputMessages::new(
                    UserMessage {
                        content: vec![
                            UserContent::Image(Image {
                                data: MediaData::Url(
                                    Url::parse("https://example.com/image.png")
                                        .expect("URL should parse"),
                                ),
                                media_type: "image/png".to_string(),
                            }),
                            UserContent::Audio(Audio {
                                data: MediaData::Base64("YXVkaW8=".to_string()),
                                media_type: "audio/mpeg".to_string(),
                            }),
                        ],
                    }
                    .to_gen_ai_type()
                    .expect("input message should convert"),
                )
                .to_json()
                .expect("input messages should serialize"),
            )
            .expect("input JSON should parse"),
            json!([
                {
                    "role": "user",
                    "parts": [
                        {
                            "type": "uri",
                            "mime_type": "image/png",
                            "modality": "image",
                            "uri": "https://example.com/image.png"
                        },
                        {
                            "type": "blob",
                            "mime_type": "audio/mpeg",
                            "modality": "audio",
                            "content": "YXVkaW8="
                        }
                    ]
                }
            ]),
        );
    }

    #[test]
    fn input_messages_include_only_visible_reasoning() {
        assert_eq!(
            from_str::<Value>(
                &GenAiInputMessages::new(
                    [
                        ChatMessage::Assistant(AssistantMessage {
                            id: None,
                            content: vec![AssistantContent::Reasoning(Reasoning {
                                id: Some("reasoning-1".to_string()),
                                content: vec![
                                    ReasoningContent::Text {
                                        text: "visible".to_string(),
                                        signature: Some("signature".to_string()),
                                    },
                                    ReasoningContent::Encrypted("encrypted".to_string()),
                                    ReasoningContent::Redacted("redacted".to_string()),
                                    ReasoningContent::Summary("summary".to_string()),
                                ],
                            })],
                        })
                        .to_gen_ai_type()
                        .expect("assistant message should convert"),
                        UserMessage {
                            content: vec![UserContent::Text("continue".to_string())],
                        }
                        .to_gen_ai_type()
                        .expect("user message should convert"),
                    ]
                    .into_iter()
                    .flatten(),
                )
                .to_json()
                .expect("input messages should serialize"),
            )
            .expect("input JSON should parse"),
            json!([
                {
                    "role": "assistant",
                    "parts": [
                        {
                            "type": "reasoning",
                            "content": "visible"
                        },
                        {
                            "type": "reasoning",
                            "content": "summary"
                        }
                    ]
                },
                {
                    "role": "user",
                    "parts": [
                        {
                            "type": "text",
                            "content": "continue"
                        }
                    ]
                }
            ]),
        );
    }

    #[test]
    fn output_coalesces_deltas_and_uses_complete_content() {
        let mut output = GenAiOutput::default();

        for event in [
            CompletionStreamEvent::TextDelta { delta: "hel".to_string() },
            CompletionStreamEvent::TextDelta { delta: "lo".to_string() },
            CompletionStreamEvent::ReasoningDelta {
                id: Some("reasoning-1".to_string()),
                delta: "draft".to_string(),
            },
            CompletionStreamEvent::Reasoning(Reasoning {
                id: Some("reasoning-1".to_string()),
                content: vec![
                    ReasoningContent::Text {
                        text: "final".to_string(),
                        signature: Some("signature".to_string()),
                    },
                    ReasoningContent::Summary("summary".to_string()),
                ],
            }),
            CompletionStreamEvent::ToolCallDelta {
                id: "call-1".to_string(),
                delta: ToolCallDelta::Arguments("{\"value\":".to_string()),
            },
            CompletionStreamEvent::ToolCall(ToolCall {
                id: "call-1".to_string(),
                name: "calculate".to_string(),
                arguments: json!({"value": 42}),
            }),
        ] {
            output.observe(&event);
        }

        assert_eq!(
            from_str::<Value>(
                &output
                    .to_json(&CompletionStreamFinal {
                        usage: TokenUsage {
                            input_tokens: 3,
                            output_tokens: 5,
                            cache_input_tokens: None,
                        },
                        finish_reason: None,
                    })
                    .expect("output should serialize"),
            )
            .expect("output JSON should parse"),
            json!([
                {
                    "role": "assistant",
                    "parts": [
                        {
                            "type": "text",
                            "content": "hello"
                        },
                        {
                            "type": "reasoning",
                            "content": "final"
                        },
                        {
                            "type": "reasoning",
                            "content": "summary"
                        },
                        {
                            "type": "tool_call",
                            "id": "call-1",
                            "name": "calculate",
                            "arguments": {
                                "value": 42
                            }
                        }
                    ],
                    "finish_reason": "tool_call"
                }
            ]),
        );
    }

    #[test]
    fn output_finish_reason_uses_provider_then_observed_content() {
        let mut output = GenAiOutput::default();
        output.observe(&CompletionStreamEvent::ToolCall(ToolCall {
            id: "call-1".to_string(),
            name: "calculate".to_string(),
            arguments: json!({}),
        }));

        assert_eq!(
            from_str::<Value>(
                &output
                    .to_json(&CompletionStreamFinal {
                        usage: TokenUsage {
                            input_tokens: 0,
                            output_tokens: 0,
                            cache_input_tokens: None,
                        },
                        finish_reason: Some("stop".to_string()),
                    })
                    .expect("output should serialize"),
            )
            .expect("output JSON should parse")[0]["finish_reason"],
            json!("stop"),
        );
        assert_eq!(
            from_str::<Value>(
                &output
                    .to_json(&CompletionStreamFinal {
                        usage: TokenUsage {
                            input_tokens: 0,
                            output_tokens: 0,
                            cache_input_tokens: None,
                        },
                        finish_reason: None,
                    })
                    .expect("output should serialize"),
            )
            .expect("output JSON should parse")[0]["finish_reason"],
            json!("tool_call"),
        );
        assert_eq!(
            from_str::<Value>(
                &GenAiOutput::default()
                    .to_json(&CompletionStreamFinal {
                        usage: TokenUsage {
                            input_tokens: 0,
                            output_tokens: 0,
                            cache_input_tokens: None,
                        },
                        finish_reason: None,
                    })
                    .expect("output should serialize"),
            )
            .expect("output JSON should parse")[0]["finish_reason"],
            json!("unknown"),
        );
    }

    #[tokio::test]
    async fn adapter_yields_the_inner_events_unchanged() {
        let inner = ChatCompletionStream::new(stream::iter(vec![
            Ok(CompletionStreamEvent::TextDelta { delta: "hi".to_string() }),
            Ok(CompletionStreamEvent::Done(CompletionStreamFinal {
                usage: TokenUsage {
                    input_tokens: 3,
                    output_tokens: 5,
                    cache_input_tokens: None,
                },
                finish_reason: None,
            })),
        ]));

        let collected = InstrumentedStream::new(inner, Span::none(), Vec::new(), Instant::now())
            .map(|event| event.expect("event"))
            .collect::<Vec<_>>()
            .await;

        assert!(matches!(
            collected.as_slice(),
            [
                CompletionStreamEvent::TextDelta { .. },
                CompletionStreamEvent::Done(_),
            ]
        ));
    }
}
