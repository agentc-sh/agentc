// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::Serialize;
use serde_json::Value;

/// Media modalities supported by OpenTelemetry GenAI message parts.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GenAiModality {
    Image,
    Audio,
    Video,
    Document,
}

/// A structured part of an OpenTelemetry GenAI message.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GenAiPart(GenAiPartValue);

impl GenAiPart {
    /// Creates a text part.
    pub fn text(content: impl Into<String>) -> Self {
        Self(GenAiPartValue::Text {
            content: content.into(),
        })
    }

    /// Creates a tool-call part.
    pub fn tool_call(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: Value,
    ) -> Self {
        Self(GenAiPartValue::ToolCall {
            id: id.into(),
            name: name.into(),
            arguments,
        })
    }

    /// Creates a tool-call response part.
    pub fn tool_call_response(
        id: impl Into<String>,
        response: Value,
    ) -> Self {
        Self(GenAiPartValue::ToolCallResponse {
            id: id.into(),
            response,
        })
    }

    /// Creates an inline media part.
    pub fn blob(
        mime_type: impl Into<String>,
        modality: GenAiModality,
        content: impl Into<String>,
    ) -> Self {
        Self(GenAiPartValue::Blob {
            mime_type: mime_type.into(),
            modality,
            content: content.into(),
        })
    }

    /// Creates a URI media part.
    pub fn uri(
        mime_type: impl Into<String>,
        modality: GenAiModality,
        uri: impl Into<String>,
    ) -> Self {
        Self(GenAiPartValue::Uri {
            mime_type: mime_type.into(),
            modality,
            uri: uri.into(),
        })
    }

    /// Creates a visible reasoning part.
    pub fn reasoning(content: impl Into<String>) -> Self {
        Self(GenAiPartValue::Reasoning {
            content: content.into(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum GenAiPartValue {
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
        modality: GenAiModality,
        content: String,
    },
    Uri {
        mime_type: String,
        modality: GenAiModality,
        uri: String,
    },
    Reasoning {
        content: String,
    },
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::semconv::genai::{GenAiModality, GenAiPart};

    #[test]
    fn media_parts_preserve_location_and_metadata() {
        assert_eq!(
            serde_json::to_value([
                GenAiPart::uri(
                    "image/png",
                    GenAiModality::Image,
                    "https://example.com/image.png",
                ),
                GenAiPart::blob(
                    "audio/mpeg",
                    GenAiModality::Audio,
                    "base64-content",
                ),
            ])
            .unwrap(),
            json!([
                {
                    "type": "uri",
                    "mime_type": "image/png",
                    "modality": "image",
                    "uri": "https://example.com/image.png",
                },
                {
                    "type": "blob",
                    "mime_type": "audio/mpeg",
                    "modality": "audio",
                    "content": "base64-content",
                },
            ]),
        );
    }
}
