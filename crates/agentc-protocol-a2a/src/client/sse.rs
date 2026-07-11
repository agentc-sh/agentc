// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::str;
use async_trait::async_trait;
use async_stream::try_stream;
use futures::stream::{Stream, StreamExt};
use reqwest::Response;

use crate::client::A2aClientError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub event_type: Option<String>,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Event(Event),
    Comment(String),
}

impl Item {
    pub fn event(event: impl Into<Event>) -> Self {
        Item::Event(event.into())
    }

    pub fn comment(comment: impl Into<String>) -> Self {
        Item::Comment(comment.into())
    }
}

#[derive(Debug, Default)]
pub struct Decoder {
    buffer: Vec<u8>,
    event_type: Option<String>,
    data: Vec<String>,
    id: Option<String>,
    retry: Option<u64>,
}

impl Decoder {
    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<Item>, A2aClientError> {
        self.buffer.extend_from_slice(chunk);

        let mut items = Vec::new();

        while let Some(index) = self
            .buffer
            .iter()
            .position(|byte| *byte == b'\n')
        {
            let mut line = self
                .buffer
                .drain(..=index)
                .collect::<Vec<_>>();

            if line.last() == Some(&b'\n') {
                line.pop();
            }

            if line.last() == Some(&b'\r') {
                line.pop();
            }

            if let Some(item) = self.push_line(str::from_utf8(&line).map_err(|err| {
                A2aClientError::stream_decode(err.to_string())
            })?)? {
                items.push(item);
            }
        }

        Ok(items)
    }

    pub fn finish(&mut self) -> Result<Vec<Item>, A2aClientError> {
        let mut items = Vec::new();

        if !self.buffer.is_empty() {
            let line = std::mem::take(&mut self.buffer);

            if let Some(item) = self.push_line(str::from_utf8(&line).map_err(|err| {
                A2aClientError::stream_decode(err.to_string())
            })?)? {
                items.push(item);
            }
        }

        if let Some(item) = self.flush_event() {
            items.push(item);
        }

        Ok(items)
    }

    fn push_line(&mut self, line: &str) -> Result<Option<Item>, A2aClientError> {
        if line.is_empty() {
            return Ok(self.flush_event());
        }

        if let Some(comment) = line.strip_prefix(':') {
            return Ok(Some(Item::comment(comment)));
        }

        let (field, value) = line
            .split_once(':')
            .map(|(field, value)| {
                (
                    field,
                    value
                        .strip_prefix(' ')
                        .unwrap_or(value),
                )
            })
            .unwrap_or((line, ""));

        match field {
            "event" => self.event_type = Some(value.to_string()),
            "data" => self.data.push(value.to_string()),
            "id" if !value.contains('\0') => self.id = Some(value.to_string()),
            "retry" => {
                if value
                    .chars()
                    .all(|character| character.is_ascii_digit())
                {
                    self.retry = value.parse().ok();
                }
            },
            _ => {},
        }

        Ok(None)
    }

    fn flush_event(&mut self) -> Option<Item> {
        if self.data.is_empty() {
            self.event_type = None;
            self.retry = None;
            return None;
        }

        Some(Item::event(Event {
            event_type: self.event_type.take(),
            data: self.data
                .drain(..)
                .collect::<Vec<_>>()
                .join("\n"),
            id: self.id.clone(),
            retry: self.retry.take(),
        }))
    }
}

#[async_trait]
pub trait Sse {
    fn sse(self) -> impl Stream<Item = Result<Item, A2aClientError>>;
}

impl Sse for Response {
    fn sse(self) -> impl Stream<Item = Result<Item, A2aClientError>> {
        try_stream! {
            let mut decoder = Decoder::default();
            let mut stream = self.bytes_stream();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|err| A2aClientError::stream_decode(err.to_string()))?;
                let items = decoder.push(&chunk)?;

                for item in items {
                    yield item;
                }
            }

            let items = decoder.finish()?;

            for item in items {
                yield item;
            }
        }
    }
}
