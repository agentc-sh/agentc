// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::guestjs::{
    errors::Error,
    handle::{BoundObject, Object, Value},
    host::Args,
    marshal::ToGuest,
    runtime::Scope,
    value::JsValue,
};
use bytes::Bytes;

use crate::typescript::{dirent::Dirent, encoding::Encoding};

#[derive(Clone)]
pub(crate) enum BufferValue {
    Bytes(Bytes),
    Value(Value),
    Text(String),
}

impl ToGuest for BufferValue {
    fn to_guest<'js>(self, scope: &Scope<'js>) -> Result<JsValue<'js>, Error> {
        match self {
            Self::Bytes(bytes) => bytes.to_guest(scope),
            Self::Value(value) => value.to_guest(scope),
            Self::Text(text) => text.to_guest(scope),
        }
    }
}

pub(crate) struct BufferView;

impl BufferView {
    fn constructor_name<'js>(object: &BoundObject<'js>) -> Result<String, Error> {
        object
            .get::<Object>("constructor")?
            .get::<String>("name")
    }

    pub(crate) fn checked_length(length: usize, offset: usize) -> Result<usize, Error> {
        length
            .checked_sub(offset)
            .ok_or_else(|| Error::conversion("agentc:fs: buffer offset is out of range"))
    }

    pub(crate) fn is_uint8_array<'js>(object: &BoundObject<'js>) -> Result<bool, Error> {
        Ok(Self::constructor_name(object)? == "Uint8Array")
    }

    pub(crate) fn len<'js>(object: &BoundObject<'js>) -> Result<usize, Error> {
        Ok(object.get::<u32>("length")? as usize)
    }

    pub(crate) fn read<'js>(
        object: &BoundObject<'js>,
        offset: usize,
        length: usize,
    ) -> Result<Vec<u8>, Error> {
        let end = offset + length;

        if end > Self::len(object)? {
            return Err(Error::conversion("agentc:fs: buffer read exceeds the view length"));
        }

        let mut bytes = Vec::with_capacity(length);

        for index in offset..end {
            bytes.push(object.get::<u32>(&index.to_string())? as u8);
        }

        Ok(bytes)
    }

    pub(crate) fn write<'js>(
        object: &BoundObject<'js>,
        offset: usize,
        bytes: &[u8],
    ) -> Result<(), Error> {
        let end = offset + bytes.len();

        if end > Self::len(object)? {
            return Err(Error::conversion("agentc:fs: buffer write exceeds the view length"));
        }

        for (index, byte) in bytes.iter().enumerate() {
            object.set(&(offset + index).to_string(), *byte as u32)?;
        }

        Ok(())
    }
}

pub(crate) enum FileContents {
    Text(String),
    Bytes(Bytes),
}

impl ToGuest for FileContents {
    fn to_guest<'js>(self, scope: &Scope<'js>) -> Result<JsValue<'js>, Error> {
        match self {
            Self::Text(text) => text.to_guest(scope),
            Self::Bytes(bytes) => bytes.to_guest(scope),
        }
    }
}

pub(crate) enum FileData {
    Text(String),
    Bytes(Bytes),
}

impl FileData {
    pub(crate) fn from_args<'js>(
        scope: &Scope<'js>,
        args: &Args<'js>,
        index: usize,
    ) -> Result<Self, Error> {
        // A `Uint8Array` is not an `Array`, so it is read as bytes first or serde would see a map.
        if let Ok(Some(bytes)) = args.get_opt::<Bytes>(scope, index) {
            return Ok(Self::Bytes(bytes));
        }

        Ok(Self::Text(args.get_owned::<String>(scope, index)?))
    }

    pub(crate) fn into_bytes(self, encoding: Option<Encoding>) -> Result<Vec<u8>, Error> {
        match self {
            Self::Text(text) => encoding
                .unwrap_or(Encoding::Utf8)
                .encode(&text),
            Self::Bytes(bytes) => Ok(bytes.to_vec()),
        }
    }
}

pub(crate) struct WriteBuffer {
    pub(crate) bytes: Vec<u8>,
    pub(crate) position: Option<u64>,
}

impl WriteBuffer {
    pub(crate) fn from_args<'js>(
        scope: &Scope<'js>,
        args: &Args<'js>,
        offset_index: usize,
    ) -> Result<Self, Error> {
        let buffer = args.get::<Object>(scope, 1)?;
        let offset = args
            .get_opt::<u32>(scope, offset_index)?
            .unwrap_or_default() as usize;
        let length = args
            .get_opt::<u32>(scope, offset_index + 1)?
            .map(|length| length as usize)
            .unwrap_or(BufferView::checked_length(BufferView::len(&buffer)?, offset)?);

        Ok(Self {
            bytes: BufferView::read(&buffer, offset, length)?,
            position: args.get_opt::<u64>(scope, offset_index + 2)?,
        })
    }
}

pub(crate) enum ReaddirResult {
    Entries(Vec<Dirent>),
    Names(Vec<String>),
}

impl ToGuest for ReaddirResult {
    fn to_guest<'js>(self, scope: &Scope<'js>) -> Result<JsValue<'js>, Error> {
        match self {
            Self::Entries(entries) => entries.to_guest(scope),
            Self::Names(names) => names.to_guest(scope),
        }
    }
}
