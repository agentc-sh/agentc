#![allow(unused)]

// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use config::{ConfigBuilder, Environment, File, builder::AsyncState};
use sanitizer::Sanitizer;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use validator::Validate;

use crate::parser::{errors::ParserError, format::SpecFormat};

/// A utility for parsing a specification from various sources (e.g. file, environment variables) and validating it.
pub struct SpecParser<T> {
    builder: ConfigBuilder<AsyncState>,
    _marker: PhantomData<T>,
}

impl<T> SpecParser<T>
where
    T: DeserializeOwned + Validate + Sanitizer,
{
    pub fn new() -> Self {
        Self {
            builder: ConfigBuilder::<AsyncState>::default(),
            _marker: PhantomData,
        }
    }

    /// Add a file path as one of the sources for the specification.
    /// The file will be parsed according to its extension (e.g. .json, .yaml).
    ///
    /// If the file does not exist or cannot be parsed, an error will be returned when `parse()` is called.
    pub fn with_file(mut self, path: impl AsRef<str>) -> Self {
        self.builder = self
            .builder
            .add_source(File::with_name(path.as_ref()));
        self
    }

    /// Add a file path as one of the sources for the specification, specifying the format explicitly.
    /// This is useful when the file extension does not match the actual format (e.g. a .txt file containing JSON),
    /// or if you want to include custom formatting middleware with the [`SpecFormat`](crate::parser::format::SpecFormat).
    pub fn with_file_format(mut self, path: impl AsRef<str>, format: SpecFormat) -> Self {
        self.builder = self
            .builder
            .add_source(File::new(path.as_ref(), format));
        self
    }

    /// Conditionally add a file source if the path is `Some`. If the path is `None`, this method does nothing.
    pub fn with_optional_file(self, path: Option<impl AsRef<str>>) -> Self {
        match path {
            Some(p) => self.with_file(p),
            None => self,
        }
    }

    pub fn with_content(mut self, content: impl AsRef<str>, format: SpecFormat) -> Self {
        self.builder = self
            .builder
            .add_source(File::from_str(content.as_ref(), format));
        self
    }

    /// Add environment variables as a source for the specification with the given prefix.
    pub fn with_env(mut self, prefix: impl AsRef<str>) -> Self {
        self.builder = self.builder.add_source(
            Environment::with_prefix(prefix.as_ref())
                .separator("__")
                .try_parsing(true),
        );
        self
    }

    /// Parse the specification from the configured sources and validate it.
    pub async fn parse(self) -> Result<T, ParserError> {
        let mut spec = self
            .builder
            .build()
            .await?
            .try_deserialize::<T>()?;

        spec.sanitize();
        spec.validate()?;

        Ok(spec)
    }
}

impl<T> Default for SpecParser<T>
where
    T: DeserializeOwned + Validate + Sanitizer,
{
    fn default() -> Self {
        Self::new()
    }
}
