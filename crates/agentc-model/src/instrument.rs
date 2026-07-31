// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#![allow(deprecated)]

use async_trait::async_trait;
use futures::Stream;
use serde::Serialize;
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
    semconv::{attribute, metric},
};

use crate::{
    errors::ModelError,
    stream::ChatCompletionStream,
    traits::CompletionModel,
    types::{
        identity::{ModelId, ProviderId},
        inference::InferenceParams,
        media::MediaData,
        message::{AssistantContent, AssistantMessage, ChatMessage, UserContent, UserMessage},
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

#[derive(Serialize)]
struct GenAiInputMessages(Vec<GenAiMessage>);

impl GenAiInputMessages {
    fn new(history: &[&ChatMessage], latest: &UserMessage) -> Result<Self, serde_json::Error> {
        let mut messages = Vec::new();

        for message in history {
            match *message {
                ChatMessage::User(message) => Self::push_user(&mut messages, message)?,
                ChatMessage::Assistant(message) => messages.push(GenAiMessage::assistant(message)),
                ChatMessage::System(_) => {}
            }
        }

        Self::push_user(&mut messages, latest)?;

        Ok(Self(messages))
    }

    fn push_user(
        messages: &mut Vec<GenAiMessage>,
        message: &UserMessage,
    ) -> Result<(), serde_json::Error> {
        let mut parts = Vec::new();

        for content in &message.content {
            match content {
                UserContent::Text(content) => {
                    parts.push(GenAiPart::Text { content: content.clone() });
                }
                UserContent::ToolResult(result) => {
                    Self::push_message(messages, GenAiRole::User, std::mem::take(&mut parts));
                    messages.push(GenAiMessage {
                        role: GenAiRole::Tool,
                        parts: vec![GenAiPart::tool_call_response(result)?],
                    });
                }
                UserContent::Image(image) => {
                    parts.push(GenAiPart::media("image", &image.data, &image.media_type));
                }
                UserContent::Audio(audio) => {
                    parts.push(GenAiPart::media("audio", &audio.data, &audio.media_type));
                }
                UserContent::Video(video) => {
                    parts.push(GenAiPart::media("video", &video.data, &video.media_type));
                }
                UserContent::Document(document) => {
                    parts.push(GenAiPart::media("document", &document.data, &document.media_type));
                }
            }
        }

        Self::push_message(messages, GenAiRole::User, parts);

        Ok(())
    }

    fn push_message(messages: &mut Vec<GenAiMessage>, role: GenAiRole, parts: Vec<GenAiPart>) {
        if !parts.is_empty() {
            messages.push(GenAiMessage { role, parts });
        }
    }

    fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[derive(Serialize)]
struct GenAiMessage {
    role: GenAiRole,
    parts: Vec<GenAiPart>,
}

impl GenAiMessage {
    fn assistant(message: &AssistantMessage) -> Self {
        let mut parts = Vec::new();

        for content in &message.content {
            match content {
                AssistantContent::Text(content) => {
                    parts.push(GenAiPart::Text { content: content.clone() });
                }
                AssistantContent::ToolCall(call) => {
                    parts.push(GenAiPart::tool_call(call));
                }
                AssistantContent::Reasoning(reasoning) => {
                    parts.extend(
                        GenAiPart::reasoning_content(reasoning)
                            .into_iter()
                            .map(|content| GenAiPart::Reasoning { content }),
                    );
                }
                AssistantContent::Image(image) => {
                    parts.push(GenAiPart::media("image", &image.data, &image.media_type));
                }
            }
        }

        Self { role: GenAiRole::Assistant, parts }
    }
}

#[derive(Serialize)]
struct GenAiOutputMessage {
    role: GenAiRole,
    parts: Vec<GenAiPart>,
    finish_reason: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum GenAiRole {
    User,
    Assistant,
    Tool,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum GenAiPart {
    Text {
        content: String,
    },
    ToolCall {
        id: String,
        name: String,
        arguments: Value,
    },
    ToolCallResponse {
        id: String,
        response: Value,
    },
    Blob {
        mime_type: String,
        modality: &'static str,
        content: String,
    },
    Uri {
        mime_type: String,
        modality: &'static str,
        uri: String,
    },
    Reasoning {
        content: String,
    },
}

impl GenAiPart {
    fn media(modality: &'static str, data: &MediaData, mime_type: &str) -> Self {
        match data {
            MediaData::Base64(content) => Self::Blob {
                mime_type: mime_type.to_string(),
                modality,
                content: content.clone(),
            },
            MediaData::Url(uri) => Self::Uri {
                mime_type: mime_type.to_string(),
                modality,
                uri: uri.to_string(),
            },
        }
    }

    fn tool_call(call: &ToolCall) -> Self {
        Self::ToolCall {
            id: call.id.clone(),
            name: call.name.clone(),
            arguments: call.arguments.clone(),
        }
    }

    fn tool_call_response(result: &ToolResult) -> Result<Self, serde_json::Error> {
        Ok(Self::ToolCallResponse {
            id: result.call_id.clone(),
            response: match result.content.as_slice() {
                [] => Value::Array(Vec::new()),
                [content] => Self::tool_result_content(content)?,
                contents => Value::Array(
                    contents
                        .iter()
                        .map(Self::tool_result_content)
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            },
        })
    }

    fn tool_result_content(content: &ToolResultContent) -> Result<Value, serde_json::Error> {
        match content {
            ToolResultContent::Text(content) => Ok(Value::String(content.clone())),
            ToolResultContent::Image(image) => {
                serde_json::to_value(Self::media("image", &image.data, &image.media_type))
            }
        }
    }

    fn reasoning_content(reasoning: &Reasoning) -> Vec<String> {
        reasoning
            .content
            .iter()
            .filter_map(|content| match content {
                ReasoningContent::Text { text, .. } => Some(text.clone()),
                ReasoningContent::Summary(summary) => Some(summary.clone()),
                ReasoningContent::Encrypted(_) | ReasoningContent::Redacted(_) => None,
            })
            .collect()
    }

    fn system_instructions(content: &str) -> Result<String, serde_json::Error> {
        serde_json::to_string(&[Self::Text { content: content.to_string() }])
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
                        content: GenAiPart::reasoning_content(reasoning),
                        complete: true,
                    };
                } else {
                    self.parts
                        .push(GenAiOutputPart::Reasoning {
                            id: reasoning.id.clone(),
                            content: GenAiPart::reasoning_content(reasoning),
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
        serde_json::to_string(&[GenAiOutputMessage {
            role: GenAiRole::Assistant,
            parts: self.message_parts(),
            finish_reason: final_response
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
        }])
    }

    fn message_parts(&self) -> Vec<GenAiPart> {
        let mut parts = Vec::new();

        for part in &self.parts {
            match part {
                GenAiOutputPart::Text(content) if !content.is_empty() => {
                    parts.push(GenAiPart::Text { content: content.clone() });
                }
                GenAiOutputPart::Reasoning { content, .. } => {
                    parts.extend(
                        content
                            .iter()
                            .filter(|content| !content.is_empty())
                            .map(|content| GenAiPart::Reasoning { content: content.clone() }),
                    );
                }
                GenAiOutputPart::ToolCall(call) => {
                    parts.push(GenAiPart::tool_call(call));
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
            && let Ok(content) = GenAiPart::system_instructions(system)
        {
            span.record(attribute::GEN_AI_SYSTEM_INSTRUCTIONS, content.as_str());
        }

        if let Ok(content) =
            GenAiInputMessages::new(&rest, latest).and_then(|messages| messages.to_json())
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
    use super::*;

    use futures::{StreamExt, stream};
    use serde_json::{from_str, json};
    use url::Url;

    use crate::types::{
        media::{Audio, Image},
        message::ChatHistory,
        stream::ToolCallDelta,
        usage::TokenUsage,
    };

    #[test]
    fn input_messages_preserve_user_and_tool_result_order() {
        assert_eq!(
            from_str::<Value>(
                &GenAiInputMessages::new(
                    &[&ChatMessage::User(UserMessage {
                        content: vec![
                            UserContent::Text("before".to_string()),
                            UserContent::ToolResult(ToolResult {
                                call_id: "call-1".to_string(),
                                content: vec![ToolResultContent::Text("result".to_string(),)],
                            }),
                            UserContent::Text("after".to_string()),
                        ],
                    })],
                    &UserMessage {
                        content: vec![UserContent::Text("latest".to_string())],
                    },
                )
                .expect("input messages should convert")
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
                &GenAiPart::system_instructions(
                    system
                        .as_deref()
                        .expect("system instructions should exist"),
                )
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
                &GenAiInputMessages::new(&rest, latest)
                    .expect("input messages should convert")
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
                    &[],
                    &UserMessage {
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
                    },
                )
                .expect("input messages should convert")
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
                    &[&ChatMessage::Assistant(AssistantMessage {
                        id: None,
                        content: vec![AssistantContent::Reasoning(Reasoning {
                            id: Some("reasoning-1".to_string()),
                            content: vec![
                                ReasoningContent::Text {
                                    text: "visible".to_string(),
                                    signature: Some("signature".to_string()),
                                },
                                ReasoningContent::Encrypted("encrypted".to_string(),),
                                ReasoningContent::Redacted("redacted".to_string(),),
                                ReasoningContent::Summary("summary".to_string(),),
                            ],
                        })],
                    })],
                    &UserMessage {
                        content: vec![UserContent::Text("continue".to_string())],
                    },
                )
                .expect("input messages should convert")
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
