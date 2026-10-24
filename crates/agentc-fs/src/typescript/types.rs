// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::{
    backend::ExecutorBackend,
    guestjs::{
        FromGuest, ToGuest, enter::Enter, errors::Error, handle::buffer::Uint8Array,
        marshal::args::Args,
    },
};
use bytes::Bytes;

use crate::typescript::{dirent::Dirent, encoding::Encoding};

#[derive(ToGuest)]
#[guestjs(union, backend = B, crate_path = agentc_executor_typescript::guestjs)]
pub(crate) enum BufferValue<B: ExecutorBackend> {
    Bytes(Bytes),
    Value(Uint8Array<B>),
    Text(String),
}

impl<B: ExecutorBackend> Clone for BufferValue<B> {
    fn clone(&self) -> Self {
        match self {
            Self::Bytes(bytes) => Self::Bytes(bytes.clone()),
            Self::Value(value) => Self::Value(value.clone()),
            Self::Text(text) => Self::Text(text.clone()),
        }
    }
}

pub(crate) struct BufferRange<B: ExecutorBackend> {
    pub(crate) buffer: Uint8Array<B>,
    pub(crate) offset: usize,
    pub(crate) length: usize,
}

impl<B: ExecutorBackend> BufferRange<B> {
    pub(crate) fn new(
        buffer: Uint8Array<B>,
        offset: usize,
        length: Option<u32>,
    ) -> Result<Self, Error> {
        let length = match length {
            Some(length) => length as usize,
            None => Self::checked_length(buffer.len()?, offset)?,
        };

        Ok(
            Self {
                buffer,
                offset,
                length,
            }
        )
    }

    pub(crate) fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Self::new(
            args.next::<Uint8Array<B>>(enter)?,
            args.next::<Option<u32>>(enter)?
                .unwrap_or_default() as usize,
            args.next::<Option<u32>>(enter)?,
        )
    }

    fn checked_length(length: usize, offset: usize) -> Result<usize, Error> {
        length
            .checked_sub(offset)
            .ok_or_else(|| Error::unsupported("agentc:fs: buffer offset is out of range"))
    }

    pub(crate) fn bytes(&self) -> Result<Vec<u8>, Error> {
        self.buffer
            .borrow_with(|bytes| {
                bytes
                    .get(self.offset..self.offset + self.length)
                    .map(<[u8]>::to_vec)
            })?
            .ok_or_else(|| Error::unsupported("agentc:fs: buffer read exceeds the view length"))
    }
}

#[derive(ToGuest)]
#[guestjs(union, crate_path = agentc_executor_typescript::guestjs)]
pub(crate) enum FileContents {
    Text(String),
    Bytes(Bytes),
}

#[derive(FromGuest)]
#[guestjs(union, crate_path = agentc_executor_typescript::guestjs)]
pub(crate) enum FileData {
    Bytes(Bytes),
    Text(String),
}

impl FileData {
    pub(crate) fn into_bytes(self, encoding: Option<Encoding>) -> Result<Vec<u8>, Error> {
        match self {
            Self::Text(text) => encoding
                .unwrap_or(Encoding::Utf8)
                .encode(&text),
            Self::Bytes(bytes) => Ok(bytes.to_vec()),
        }
    }
}

#[derive(ToGuest)]
#[guestjs(union, crate_path = agentc_executor_typescript::guestjs)]
pub(crate) enum ReaddirResult {
    Entries(Vec<Dirent>),
    Names(Vec<String>),
}
