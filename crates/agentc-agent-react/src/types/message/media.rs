// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use url::Url;

use agentc_agent::types::conversion::{FromModelType, ToModelType};
use agentc_model::types::media::{
    Audio as ModelAudio, Document as ModelDocument, Image as ModelImage,
    MediaData as ModelMediaData, Video as ModelVideo,
};

/// The source of a media value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum MediaSource {
    /// A URL pointing to the media.
    Url(Url),
    /// Base64-encoded media data.
    Base64(String),
}

impl ToModelType for MediaSource {
    type ModelType = ModelMediaData;

    fn to_model_type(&self) -> Self::ModelType {
        match self {
            MediaSource::Url(url) => ModelMediaData::Url(url.clone()),
            MediaSource::Base64(data) => ModelMediaData::Base64(data.clone()),
        }
    }
}

impl FromModelType for MediaSource {
    type ModelType = ModelMediaData;
    type Output = Self;

    fn from_model_type(model: Self::ModelType) -> Self::Output {
        match model {
            ModelMediaData::Url(url) => MediaSource::Url(url),
            ModelMediaData::Base64(data) => MediaSource::Base64(data),
        }
    }
}

/// An image included in a user message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Image {
    pub source: MediaSource,
    pub media_type: String,
}

impl ToModelType for Image {
    type ModelType = ModelImage;

    fn to_model_type(&self) -> Self::ModelType {
        ModelImage {
            data: self.source.to_model_type(),
            media_type: self.media_type.clone(),
        }
    }
}

impl FromModelType for Image {
    type ModelType = ModelImage;
    type Output = Self;

    fn from_model_type(model: Self::ModelType) -> Self::Output {
        Self {
            source: MediaSource::from_model_type(model.data),
            media_type: model.media_type,
        }
    }
}

/// Audio included in a user message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Audio {
    pub source: MediaSource,
    pub media_type: String,
}

impl ToModelType for Audio {
    type ModelType = ModelAudio;

    fn to_model_type(&self) -> Self::ModelType {
        ModelAudio {
            data: self.source.to_model_type(),
            media_type: self.media_type.clone(),
        }
    }
}

impl FromModelType for Audio {
    type ModelType = ModelAudio;
    type Output = Self;

    fn from_model_type(model: Self::ModelType) -> Self::Output {
        Self {
            source: MediaSource::from_model_type(model.data),
            media_type: model.media_type,
        }
    }
}

/// A video included in a user message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Video {
    pub source: MediaSource,
    pub media_type: String,
}

impl ToModelType for Video {
    type ModelType = ModelVideo;

    fn to_model_type(&self) -> Self::ModelType {
        ModelVideo {
            data: self.source.to_model_type(),
            media_type: self.media_type.clone(),
        }
    }
}

impl FromModelType for Video {
    type ModelType = ModelVideo;
    type Output = Self;

    fn from_model_type(model: Self::ModelType) -> Self::Output {
        Self {
            source: MediaSource::from_model_type(model.data),
            media_type: model.media_type,
        }
    }
}

/// A document included in a user message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Document {
    pub source: MediaSource,
    pub media_type: String,
}

impl ToModelType for Document {
    type ModelType = ModelDocument;

    fn to_model_type(&self) -> Self::ModelType {
        ModelDocument {
            data: self.source.to_model_type(),
            media_type: self.media_type.clone(),
        }
    }
}

impl FromModelType for Document {
    type ModelType = ModelDocument;
    type Output = Self;

    fn from_model_type(model: Self::ModelType) -> Self::Output {
        Self {
            source: MediaSource::from_model_type(model.data),
            media_type: model.media_type,
        }
    }
}
