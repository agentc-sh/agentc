// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

/// A media object, which can be either raw data or a URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MediaData {
    /// Raw binary data encoded as a base64 string.
    Base64(String),
    /// A URL pointing to the media data.
    Url(String),
}

/// An image media object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub data: MediaData,
    pub media_type: String,
}

/// An audio media object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Audio {
    pub data: MediaData,
    pub media_type: String,
}

/// A video media object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Video {
    pub data: MediaData,
    pub media_type: String,
}

/// A document media object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub data: MediaData,
    pub media_type: String,
}
