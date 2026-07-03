// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use rig::{
    OneOrMany as RigOneOrMany,
    message::{
        AssistantContent as RigAssistantContent, Audio as RigAudio,
        AudioMediaType as RigAudioMediaType, Document as RigDocument,
        DocumentMediaType as RigDocumentMediaType, DocumentSourceKind as RigDocumentSourceKind,
        Image as RigImage, ImageMediaType as RigImageMediaType, MediaType as RigMediaType,
        Message as RigMessage, Reasoning as RigReasoning, ReasoningContent as RigReasoningContent,
        Text as RigText, ToolCall as RigToolCall, ToolFunction as RigToolFunction,
        ToolResult as RigToolResult, ToolResultContent as RigToolResultContent,
        UserContent as RigUserContent, Video as RigVideo, VideoMediaType as RigVideoMediaType,
    },
};

use crate::{
    errors::ModelError,
    types::{
        media::{Audio, Document, Image, MediaData, Video},
        message::{AssistantContent, AssistantMessage, ChatMessage, UserContent, UserMessage},
        reasoning::{Reasoning, ReasoningContent},
        tools::{ToolCall, ToolResult, ToolResultContent},
    },
};

trait TryIntoString {
    type Error;

    fn try_into_string(self) -> Result<String, Self::Error>;
}

trait TryIntoMediaType<T> {
    type Error;

    fn try_into_media_type(self) -> Result<T, Self::Error>;
}

impl TryIntoString for RigImageMediaType {
    type Error = ModelError;

    #[allow(unreachable_patterns)]
    fn try_into_string(self) -> Result<String, Self::Error> {
        match self {
            RigImageMediaType::JPEG => Ok("image/jpeg".to_string()),
            RigImageMediaType::PNG => Ok("image/png".to_string()),
            RigImageMediaType::GIF => Ok("image/gif".to_string()),
            RigImageMediaType::WEBP => Ok("image/webp".to_string()),
            RigImageMediaType::HEIC => Ok("image/heic".to_string()),
            RigImageMediaType::HEIF => Ok("image/heif".to_string()),
            RigImageMediaType::SVG => Ok("image/svg+xml".to_string()),
            _ => Ok("application/octet-stream".to_string()),
        }
    }
}

impl TryIntoMediaType<RigImageMediaType> for String {
    type Error = ModelError;

    fn try_into_media_type(self) -> Result<RigImageMediaType, Self::Error> {
        match self.as_str() {
            "image/jpeg" => Ok(RigImageMediaType::JPEG),
            "image/png" => Ok(RigImageMediaType::PNG),
            "image/gif" => Ok(RigImageMediaType::GIF),
            "image/webp" => Ok(RigImageMediaType::WEBP),
            "image/heic" => Ok(RigImageMediaType::HEIC),
            "image/heif" => Ok(RigImageMediaType::HEIF),
            "image/svg+xml" => Ok(RigImageMediaType::SVG),
            _ => Err(ModelError::Configuration {
                message: format!("unsupported image media type: {}", self),
            }),
        }
    }
}

impl TryIntoString for RigAudioMediaType {
    type Error = ModelError;

    #[allow(unreachable_patterns)]
    fn try_into_string(self) -> Result<String, Self::Error> {
        match self {
            RigAudioMediaType::MP3 => Ok("audio/mpeg".to_string()),
            RigAudioMediaType::WAV => Ok("audio/wav".to_string()),
            RigAudioMediaType::OGG => Ok("audio/ogg".to_string()),
            RigAudioMediaType::FLAC => Ok("audio/flac".to_string()),
            RigAudioMediaType::AAC => Ok("audio/aac".to_string()),
            _ => Ok("application/octet-stream".to_string()),
        }
    }
}

impl TryIntoMediaType<RigAudioMediaType> for String {
    type Error = ModelError;

    fn try_into_media_type(self) -> Result<RigAudioMediaType, Self::Error> {
        match self.as_str() {
            "audio/mpeg" => Ok(RigAudioMediaType::MP3),
            "audio/wav" => Ok(RigAudioMediaType::WAV),
            "audio/ogg" => Ok(RigAudioMediaType::OGG),
            "audio/flac" => Ok(RigAudioMediaType::FLAC),
            "audio/aac" => Ok(RigAudioMediaType::AAC),
            _ => Err(ModelError::Configuration {
                message: format!("unsupported audio media type: {}", self),
            }),
        }
    }
}

impl TryIntoString for RigVideoMediaType {
    type Error = ModelError;

    #[allow(unreachable_patterns)]
    fn try_into_string(self) -> Result<String, Self::Error> {
        match self {
            RigVideoMediaType::AVI => Ok("video/x-msvideo".to_string()),
            RigVideoMediaType::MP4 => Ok("video/mp4".to_string()),
            RigVideoMediaType::MPEG => Ok("video/mpeg".to_string()),
            RigVideoMediaType::MOV => Ok("video/quicktime".to_string()),
            RigVideoMediaType::WEBM => Ok("video/webm".to_string()),
            _ => Ok("application/octet-stream".to_string()),
        }
    }
}

impl TryIntoMediaType<RigVideoMediaType> for String {
    type Error = ModelError;

    fn try_into_media_type(self) -> Result<RigVideoMediaType, Self::Error> {
        match self.as_str() {
            "video/x-msvideo" => Ok(RigVideoMediaType::AVI),
            "video/mp4" => Ok(RigVideoMediaType::MP4),
            "video/mpeg" => Ok(RigVideoMediaType::MPEG),
            "video/quicktime" => Ok(RigVideoMediaType::MOV),
            "video/webm" => Ok(RigVideoMediaType::WEBM),
            _ => Err(ModelError::Configuration {
                message: format!("unsupported video media type: {}", self),
            }),
        }
    }
}

impl TryIntoString for RigDocumentMediaType {
    type Error = ModelError;

    #[allow(unreachable_patterns)]
    fn try_into_string(self) -> Result<String, Self::Error> {
        match self {
            RigDocumentMediaType::PDF => Ok("application/pdf".to_string()),
            RigDocumentMediaType::TXT => Ok("text/plain".to_string()),
            RigDocumentMediaType::RTF => Ok("application/rtf".to_string()),
            RigDocumentMediaType::HTML => Ok("text/html".to_string()),
            RigDocumentMediaType::CSS => Ok("text/css".to_string()),
            RigDocumentMediaType::MARKDOWN => Ok("text/markdown".to_string()),
            RigDocumentMediaType::CSV => Ok("text/csv".to_string()),
            RigDocumentMediaType::XML => Ok("application/xml".to_string()),
            RigDocumentMediaType::Javascript => Ok("application/javascript".to_string()),
            RigDocumentMediaType::Python => Ok("text/x-python".to_string()),
            _ => Ok("application/octet-stream".to_string()),
        }
    }
}

impl TryIntoMediaType<RigDocumentMediaType> for String {
    type Error = ModelError;

    fn try_into_media_type(self) -> Result<RigDocumentMediaType, Self::Error> {
        match self.as_str() {
            "application/pdf" => Ok(RigDocumentMediaType::PDF),
            "text/plain" => Ok(RigDocumentMediaType::TXT),
            "application/rtf" => Ok(RigDocumentMediaType::RTF),
            "text/html" => Ok(RigDocumentMediaType::HTML),
            "text/css" => Ok(RigDocumentMediaType::CSS),
            "text/markdown" => Ok(RigDocumentMediaType::MARKDOWN),
            "text/csv" => Ok(RigDocumentMediaType::CSV),
            "application/xml" => Ok(RigDocumentMediaType::XML),
            "application/javascript" => Ok(RigDocumentMediaType::Javascript),
            "text/x-python" => Ok(RigDocumentMediaType::Python),
            _ => Err(ModelError::Configuration {
                message: format!("unsupported document media type: {}", self),
            }),
        }
    }
}

impl TryIntoString for RigMediaType {
    type Error = ModelError;

    #[allow(unreachable_patterns)]
    fn try_into_string(self) -> Result<String, Self::Error> {
        match self {
            RigMediaType::Image(media_type) => media_type.try_into_string(),
            RigMediaType::Audio(media_type) => media_type.try_into_string(),
            RigMediaType::Video(media_type) => media_type.try_into_string(),
            RigMediaType::Document(media_type) => media_type.try_into_string(),
            _ => Err(ModelError::Configuration { message: "unsupported media type".into() }),
        }
    }
}

impl TryFrom<RigImage> for Image {
    type Error = ModelError;

    fn try_from(value: RigImage) -> Result<Self, Self::Error> {
        Ok(Image {
            data: value.data.try_into()?,
            media_type: value
                .media_type
                .map(TryIntoString::try_into_string)
                .transpose()?
                .unwrap_or_else(|| "application/octet-stream".to_string()),
        })
    }
}

impl TryFrom<Image> for RigImage {
    type Error = ModelError;

    fn try_from(value: Image) -> Result<Self, Self::Error> {
        Ok(RigImage {
            data: value.data.try_into()?,
            media_type: Some(value.media_type.try_into_media_type()?),
            detail: None,
            additional_params: None,
        })
    }
}

impl TryFrom<RigAudio> for Audio {
    type Error = ModelError;

    fn try_from(value: RigAudio) -> Result<Self, Self::Error> {
        Ok(Audio {
            data: value.data.try_into()?,
            media_type: value
                .media_type
                .map(TryIntoString::try_into_string)
                .transpose()?
                .unwrap_or_else(|| "application/octet-stream".to_string()),
        })
    }
}

impl TryFrom<Audio> for RigAudio {
    type Error = ModelError;

    fn try_from(value: Audio) -> Result<Self, Self::Error> {
        Ok(RigAudio {
            data: value.data.try_into()?,
            media_type: Some(value.media_type.try_into_media_type()?),
            additional_params: None,
        })
    }
}

impl TryFrom<RigVideo> for Video {
    type Error = ModelError;

    fn try_from(value: RigVideo) -> Result<Self, Self::Error> {
        Ok(Video {
            data: value.data.try_into()?,
            media_type: value
                .media_type
                .map(TryIntoString::try_into_string)
                .transpose()?
                .unwrap_or_else(|| "application/octet-stream".to_string()),
        })
    }
}

impl TryFrom<Video> for RigVideo {
    type Error = ModelError;

    fn try_from(value: Video) -> Result<Self, Self::Error> {
        Ok(RigVideo {
            data: value.data.try_into()?,
            media_type: Some(value.media_type.try_into_media_type()?),
            additional_params: None,
        })
    }
}

impl TryFrom<RigDocument> for Document {
    type Error = ModelError;

    fn try_from(value: RigDocument) -> Result<Self, Self::Error> {
        Ok(Document {
            data: value.data.try_into()?,
            media_type: value
                .media_type
                .map(TryIntoString::try_into_string)
                .transpose()?
                .unwrap_or_else(|| "application/octet-stream".to_string()),
        })
    }
}

impl TryFrom<Document> for RigDocument {
    type Error = ModelError;

    fn try_from(value: Document) -> Result<Self, Self::Error> {
        Ok(RigDocument {
            data: value.data.try_into()?,
            media_type: Some(value.media_type.try_into_media_type()?),
            additional_params: None,
        })
    }
}

impl TryFrom<RigDocumentSourceKind> for MediaData {
    type Error = ModelError;

    fn try_from(value: RigDocumentSourceKind) -> Result<Self, Self::Error> {
        match value {
            RigDocumentSourceKind::Url(url) => Ok(MediaData::Url(url)),
            RigDocumentSourceKind::Base64(data) => Ok(MediaData::Base64(data)),
            _ => Err(ModelError::Configuration {
                message: format!("unsupported document source kind: {:?}", value),
            }),
        }
    }
}

impl TryFrom<MediaData> for RigDocumentSourceKind {
    type Error = ModelError;

    fn try_from(value: MediaData) -> Result<Self, Self::Error> {
        match value {
            MediaData::Url(url) => Ok(RigDocumentSourceKind::Url(url)),
            MediaData::Base64(data) => Ok(RigDocumentSourceKind::Base64(data)),
        }
    }
}

impl TryFrom<RigToolResultContent> for ToolResultContent {
    type Error = ModelError;

    fn try_from(value: RigToolResultContent) -> Result<Self, Self::Error> {
        match value {
            RigToolResultContent::Text(text) => Ok(ToolResultContent::Text(text.text)),
            RigToolResultContent::Image(image) => Ok(ToolResultContent::Image(image.try_into()?)),
        }
    }
}

impl TryFrom<ToolResultContent> for RigToolResultContent {
    type Error = ModelError;

    fn try_from(value: ToolResultContent) -> Result<Self, Self::Error> {
        match value {
            ToolResultContent::Text(text) => Ok(RigToolResultContent::Text(RigText { text })),
            ToolResultContent::Image(image) => Ok(RigToolResultContent::Image(image.try_into()?)),
        }
    }
}

impl TryFrom<RigToolCall> for ToolCall {
    type Error = ModelError;

    fn try_from(value: RigToolCall) -> Result<Self, Self::Error> {
        Ok(ToolCall {
            id: value.id,
            name: value.function.name,
            arguments: value.function.arguments,
        })
    }
}

impl TryFrom<ToolCall> for RigToolCall {
    type Error = ModelError;

    fn try_from(value: ToolCall) -> Result<Self, Self::Error> {
        Ok(RigToolCall {
            id: value.id.clone(),
            call_id: Some(value.id),
            function: RigToolFunction {
                name: value.name,
                arguments: value.arguments,
            },
            signature: None,
            additional_params: None,
        })
    }
}

impl TryFrom<RigToolResult> for ToolResult {
    type Error = ModelError;

    fn try_from(value: RigToolResult) -> Result<Self, Self::Error> {
        Ok(ToolResult {
            call_id: value.id,
            content: value
                .content
                .into_iter()
                .map(ToolResultContent::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl TryFrom<ToolResult> for RigToolResult {
    type Error = ModelError;

    fn try_from(value: ToolResult) -> Result<Self, Self::Error> {
        Ok(RigToolResult {
            id: value.call_id.clone(),
            call_id: Some(value.call_id),
            content: RigOneOrMany::many(
                value
                    .content
                    .into_iter()
                    .map(ToolResultContent::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .map_err(|e| ModelError::Configuration {
                message: format!("failed to convert tool result content: {}", e),
            })?,
        })
    }
}

impl TryFrom<RigReasoningContent> for ReasoningContent {
    type Error = ModelError;

    fn try_from(value: RigReasoningContent) -> Result<Self, Self::Error> {
        match value {
            RigReasoningContent::Text { text, signature } => {
                Ok(ReasoningContent::Text { text, signature })
            }
            RigReasoningContent::Encrypted(data) => Ok(ReasoningContent::Encrypted(data)),
            RigReasoningContent::Redacted { data } => Ok(ReasoningContent::Redacted(data)),
            RigReasoningContent::Summary(summary) => Ok(ReasoningContent::Summary(summary)),
            _ => Err(ModelError::Configuration {
                message: format!("unsupported reasoning content type: {:?}", value),
            }),
        }
    }
}

impl TryFrom<ReasoningContent> for RigReasoningContent {
    type Error = ModelError;

    fn try_from(value: ReasoningContent) -> Result<Self, Self::Error> {
        match value {
            ReasoningContent::Text { text, signature } => {
                Ok(RigReasoningContent::Text { text, signature })
            }
            ReasoningContent::Encrypted(data) => Ok(RigReasoningContent::Encrypted(data)),
            ReasoningContent::Redacted(data) => Ok(RigReasoningContent::Redacted { data }),
            ReasoningContent::Summary(summary) => Ok(RigReasoningContent::Summary(summary)),
        }
    }
}

impl TryFrom<RigReasoning> for Reasoning {
    type Error = ModelError;

    fn try_from(value: RigReasoning) -> Result<Self, Self::Error> {
        Ok(Reasoning {
            id: value.id,
            content: value
                .content
                .into_iter()
                .map(ReasoningContent::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl TryFrom<Reasoning> for RigReasoning {
    type Error = ModelError;

    fn try_from(value: Reasoning) -> Result<Self, Self::Error> {
        let result = match value.content.as_slice() {
            [] => {
                return Err(ModelError::Configuration {
                    message: "reasoning content cannot be empty".into(),
                });
            }
            [ReasoningContent::Encrypted(data)] => RigReasoning::encrypted(data),
            [ReasoningContent::Redacted(data)] => RigReasoning::redacted(data),
            [ReasoningContent::Text { text, signature }] => {
                RigReasoning::new_with_signature(text, signature.clone())
            }
            blocks => {
                let contents = blocks
                    .iter()
                    .filter_map(|block| match block {
                        ReasoningContent::Text { text, .. } => Some(text.clone()),
                        ReasoningContent::Summary(summary) => Some(summary.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                match blocks.first() {
                    Some(ReasoningContent::Text { .. }) => RigReasoning::multi(contents),
                    _ => RigReasoning::summaries(contents),
                }
            }
        };

        Ok(result.optional_id(value.id))
    }
}

impl TryFrom<RigUserContent> for UserContent {
    type Error = ModelError;

    fn try_from(value: RigUserContent) -> Result<Self, Self::Error> {
        match value {
            RigUserContent::Text(text) => Ok(UserContent::Text(text.text)),
            RigUserContent::ToolResult(result) => Ok(UserContent::ToolResult(result.try_into()?)),
            RigUserContent::Image(image) => Ok(UserContent::Image(image.try_into()?)),
            RigUserContent::Audio(audio) => Ok(UserContent::Audio(audio.try_into()?)),
            RigUserContent::Video(video) => Ok(UserContent::Video(video.try_into()?)),
            RigUserContent::Document(document) => Ok(UserContent::Document(document.try_into()?)),
        }
    }
}

impl TryFrom<UserContent> for RigUserContent {
    type Error = ModelError;

    fn try_from(value: UserContent) -> Result<Self, Self::Error> {
        match value {
            UserContent::Text(text) => Ok(RigUserContent::Text(RigText { text })),
            UserContent::ToolResult(result) => Ok(RigUserContent::ToolResult(result.try_into()?)),
            UserContent::Image(image) => Ok(RigUserContent::Image(image.try_into()?)),
            UserContent::Audio(audio) => Ok(RigUserContent::Audio(audio.try_into()?)),
            UserContent::Video(video) => Ok(RigUserContent::Video(video.try_into()?)),
            UserContent::Document(document) => Ok(RigUserContent::Document(document.try_into()?)),
        }
    }
}

impl TryFrom<RigAssistantContent> for AssistantContent {
    type Error = ModelError;

    fn try_from(value: RigAssistantContent) -> Result<Self, Self::Error> {
        match value {
            RigAssistantContent::Text(text) => Ok(AssistantContent::Text(text.text)),
            RigAssistantContent::Reasoning(reasoning) => {
                Ok(AssistantContent::Reasoning(reasoning.try_into()?))
            }
            RigAssistantContent::ToolCall(tool_call) => {
                Ok(AssistantContent::ToolCall(tool_call.try_into()?))
            }
            _ => Err(ModelError::Configuration {
                message: format!("unsupported assistant content type: {:?}", value),
            }),
        }
    }
}

impl TryFrom<AssistantContent> for RigAssistantContent {
    type Error = ModelError;

    fn try_from(value: AssistantContent) -> Result<Self, Self::Error> {
        match value {
            AssistantContent::Text(text) => Ok(RigAssistantContent::Text(RigText { text })),
            AssistantContent::Image(image) => Ok(RigAssistantContent::Image(image.try_into()?)),
            AssistantContent::Reasoning(reasoning) => {
                Ok(RigAssistantContent::Reasoning(reasoning.try_into()?))
            }
            AssistantContent::ToolCall(tool_call) => {
                Ok(RigAssistantContent::ToolCall(tool_call.try_into()?))
            }
        }
    }
}

impl TryFrom<UserMessage> for RigMessage {
    type Error = ModelError;

    fn try_from(value: UserMessage) -> Result<Self, Self::Error> {
        Ok(RigMessage::User {
            content: RigOneOrMany::many(
                value
                    .content
                    .into_iter()
                    .map(UserContent::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .map_err(|e| ModelError::Configuration {
                message: format!("failed to convert user content: {}", e),
            })?,
        })
    }
}

impl TryFrom<AssistantMessage> for RigMessage {
    type Error = ModelError;

    fn try_from(value: AssistantMessage) -> Result<Self, Self::Error> {
        Ok(RigMessage::Assistant {
            id: value.id,
            content: RigOneOrMany::many(
                value
                    .content
                    .into_iter()
                    .map(AssistantContent::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .map_err(|e| ModelError::Configuration {
                message: format!("failed to convert assistant content: {}", e),
            })?,
        })
    }
}

impl TryFrom<RigMessage> for ChatMessage {
    type Error = ModelError;

    fn try_from(value: RigMessage) -> Result<Self, Self::Error> {
        match value {
            RigMessage::User { content } => Ok(ChatMessage::User(UserMessage {
                content: content
                    .into_iter()
                    .map(UserContent::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
            })),
            RigMessage::Assistant { id, content } => Ok(ChatMessage::Assistant(AssistantMessage {
                id,
                content: content
                    .into_iter()
                    .map(AssistantContent::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
            })),
        }
    }
}

impl TryFrom<ChatMessage> for RigMessage {
    type Error = ModelError;

    fn try_from(value: ChatMessage) -> Result<Self, Self::Error> {
        match value {
            ChatMessage::User(user) => Ok(RigMessage::User {
                content: RigOneOrMany::many(
                    user.content
                        .into_iter()
                        .map(UserContent::try_into)
                        .collect::<Result<Vec<_>, _>>()?,
                )
                .map_err(|e| ModelError::Configuration {
                    message: format!("failed to convert user content: {}", e),
                })?,
            }),
            ChatMessage::Assistant(assistant) => Ok(RigMessage::Assistant {
                id: assistant.id,
                content: RigOneOrMany::many(
                    assistant
                        .content
                        .into_iter()
                        .map(AssistantContent::try_into)
                        .collect::<Result<Vec<_>, _>>()?,
                )
                .map_err(|e| ModelError::Configuration {
                    message: format!("failed to convert assistant content: {}", e),
                })?,
            }),
            other => Err(ModelError::Configuration {
                message: format!("unsupported chat message type: {}", other.role()),
            }),
        }
    }
}
