// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::marker::PhantomData;

use agentc_executor_typescript::{
    backend::ExecutorBackend,
    guestjs::{
        FromGuest,
        backend::Val,
        enter::Enter,
        errors::Error,
        handle::{buffer::Uint8Array, object::Object, protocol::ObjectProtocol, value::Value},
        host::namespace::NamespaceDeclaration,
        host_class,
        marshal::ToGuest,
    },
};
use bytes::Bytes;

use crate::{
    fs::{Dir, Owner, Permissions},
    path::PathBuf,
    typescript::{
        descriptors::Descriptors,
        encoding::Encoding,
        options::{EncodingOrOptions, FileOptions},
        stats::Stats,
        types::{BufferRange, BufferValue, FileContents, FileData},
    },
};

pub(crate) struct ReadResult<B: ExecutorBackend> {
    bytes_read: u32,
    buffer: BufferValue<B>,
}

impl<B: ExecutorBackend> ToGuest<B> for ReadResult<B> {
    fn to_guest<'js>(self, enter: &Enter<'js, B>) -> Result<Val<'js, B>, Error> {
        let mut declaration = NamespaceDeclaration::new();

        declaration
            .constant("bytesRead", self.bytes_read)
            .constant("buffer", self.buffer);

        declaration.to_guest(enter)
    }
}

pub(crate) struct WriteResult<B: ExecutorBackend> {
    bytes_written: u32,
    buffer: BufferValue<B>,
}

impl<B: ExecutorBackend> ToGuest<B> for WriteResult<B> {
    fn to_guest<'js>(self, enter: &Enter<'js, B>) -> Result<Val<'js, B>, Error> {
        let mut declaration = NamespaceDeclaration::new();

        declaration
            .constant("bytesWritten", self.bytes_written)
            .constant("buffer", self.buffer);

        declaration.to_guest(enter)
    }
}

#[derive(FromGuest)]
#[guestjs(union, backend = B, crate_path = agentc_executor_typescript::guestjs)]
enum ReadSource<B: ExecutorBackend> {
    Buffer(Uint8Array<B>),
    Options(Object<B>),
    Other(Value<B>),
}

#[derive(FromGuest)]
#[guestjs(union, backend = B, crate_path = agentc_executor_typescript::guestjs)]
enum WriteSource<B: ExecutorBackend> {
    Buffer(Uint8Array<B>),
    Text(String),
}

#[derive(FromGuest)]
#[guestjs(union, crate_path = agentc_executor_typescript::guestjs)]
enum LengthOrEncoding {
    Length(u32),
    Encoding(String),
}

impl LengthOrEncoding {
    fn into_length(self) -> Result<u32, Error> {
        match self {
            Self::Length(length) => Ok(length),
            Self::Encoding(_) => Err(Error::conversion("number", "string")),
        }
    }

    fn into_encoding(self) -> Result<Encoding, Error> {
        match self {
            Self::Encoding(encoding) => Encoding::parse(&encoding),
            Self::Length(_) => Err(Error::conversion("string", "number")),
        }
    }
}

struct ReadRequest<B: ExecutorBackend> {
    buffer: Option<Uint8Array<B>>,
    offset: usize,
    length: usize,
    position: Option<u64>,
}

impl<B: ExecutorBackend> ReadRequest<B> {
    fn default() -> Self {
        Self {
            buffer: None,
            offset: 0,
            length: 16_384,
            position: None,
        }
    }

    fn new(
        source: Option<ReadSource<B>>,
        offset: Option<u32>,
        length: Option<u32>,
        position: Option<u64>,
    ) -> Result<Self, Error> {
        match source {
            Some(ReadSource::Buffer(buffer)) => Self::from_buffer(buffer, offset, length, position),
            Some(ReadSource::Options(options)) => Self::from_options(options),
            Some(ReadSource::Other(_)) | None => Ok(Self::default()),
        }
    }

    fn from_buffer(
        buffer: Uint8Array<B>,
        offset: Option<u32>,
        length: Option<u32>,
        position: Option<u64>,
    ) -> Result<Self, Error> {
        let BufferRange { buffer, offset, length } =
            BufferRange::new(buffer, offset.unwrap_or_default() as usize, length)?;

        Ok(Self {
            buffer: Some(buffer),
            offset,
            length,
            position,
        })
    }

    fn from_options(options: Object<B>) -> Result<Self, Error> {
        let offset = options.get::<Option<u32>, _>("offset")?;
        let length = options.get::<Option<u32>, _>("length")?;
        let position = options.get::<Option<u64>, _>("position")?;

        match options.get::<Option<Uint8Array<B>>, _>("buffer")? {
            Some(buffer) => Self::from_buffer(buffer, offset, length, position),
            None => Ok(Self {
                buffer: None,
                offset: offset.unwrap_or_default() as usize,
                length: length.unwrap_or(16_384) as usize,
                position,
            }),
        }
    }
}

struct WriteRequest<B: ExecutorBackend> {
    buffer: BufferValue<B>,
    bytes: Vec<u8>,
    position: Option<u64>,
}

impl<B: ExecutorBackend> WriteRequest<B> {
    fn new(
        source: WriteSource<B>,
        offset_or_position: Option<u64>,
        length_or_encoding: Option<LengthOrEncoding>,
        position: Option<u64>,
    ) -> Result<Self, Error> {
        match source {
            WriteSource::Buffer(buffer) => Self::from_buffer(
                buffer,
                offset_or_position,
                length_or_encoding
                    .map(LengthOrEncoding::into_length)
                    .transpose()?,
                position,
            ),
            WriteSource::Text(text) => Self::from_string(
                text,
                offset_or_position,
                length_or_encoding
                    .map(LengthOrEncoding::into_encoding)
                    .transpose()?,
            ),
        }
    }

    fn from_buffer(
        buffer: Uint8Array<B>,
        offset: Option<u64>,
        length: Option<u32>,
        position: Option<u64>,
    ) -> Result<Self, Error> {
        let offset = offset
            .map(u32::try_from)
            .transpose()
            .map_err(|_| Error::conversion("u32", "number"))?
            .unwrap_or_default() as usize;
        let range = BufferRange::new(buffer, offset, length)?;

        Ok(Self {
            bytes: range.bytes()?,
            buffer: BufferValue::Value(range.buffer),
            position,
        })
    }

    fn from_string(
        text: String,
        position: Option<u64>,
        encoding: Option<Encoding>,
    ) -> Result<Self, Error> {
        Ok(Self {
            bytes: encoding
                .unwrap_or(Encoding::Utf8)
                .encode(&text)?,
            buffer: BufferValue::Text(text),
            position,
        })
    }
}

pub struct FileHandle<B: ExecutorBackend> {
    fd: u32,
    path: PathBuf,
    dir: Dir,
    descriptors: Descriptors,
    backend: PhantomData<B>,
}

impl<B: ExecutorBackend> FileHandle<B> {
    pub fn new(fd: u32, path: PathBuf, dir: Dir, descriptors: Descriptors) -> Self {
        Self {
            fd,
            path,
            dir,
            descriptors,
            backend: PhantomData,
        }
    }
}

#[host_class(backend = B, rename_all = "camelCase", crate_path = agentc_executor_typescript::guestjs)]
impl<B: ExecutorBackend> FileHandle<B> {
    #[guestjs(get)]
    fn fd(&self) -> Result<u32, Error> {
        Ok(self.fd)
    }

    #[guestjs(async_method)]
    fn close(&self) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let descriptors = self.descriptors.clone();
        let fd = self.fd;

        Ok(async move {
            descriptors.close(fd)?;

            Ok(())
        })
    }

    #[guestjs(async_method)]
    fn stat(&self) -> Result<impl Future<Output = Result<Stats<B>, Error>> + use<B>, Error> {
        let dir = self.dir.clone();
        let path = self.path.clone();

        Ok(async move { Ok(Stats::new(dir.metadata(path).await?)) })
    }

    #[guestjs(async_method)]
    fn chmod(&self, mode: u32) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let dir = self.dir.clone();
        let path = self.path.clone();

        Ok(async move {
            dir.set_permissions(path, Permissions::new(mode))
                .await?;

            Ok(())
        })
    }

    #[guestjs(async_method)]
    fn chown(
        &self,
        uid: u32,
        gid: u32,
    ) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let dir = self.dir.clone();
        let path = self.path.clone();

        Ok(async move {
            dir.set_owner(path, Owner::new().user(uid).group(gid))
                .await?;

            Ok(())
        })
    }

    #[guestjs(async_method)]
    fn truncate(
        &self,
        len: Option<u64>,
    ) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let mut lease = self.descriptors.lease(self.fd)?;

        Ok(async move {
            lease
                .session()?
                .truncate(len.unwrap_or_default())
                .await
                .map_err(Into::into)
        })
    }

    #[guestjs(async_method)]
    fn sync(&self) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let mut lease = self.descriptors.lease(self.fd)?;

        Ok(async move {
            lease
                .session()?
                .sync_all()
                .await
                .map_err(Into::into)
        })
    }

    #[guestjs(async_method)]
    fn datasync(&self) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let mut lease = self.descriptors.lease(self.fd)?;

        Ok(async move {
            lease
                .session()?
                .sync_data()
                .await
                .map_err(Into::into)
        })
    }

    #[guestjs(async_method)]
    fn read_file(
        &self,
        options: Option<EncodingOrOptions>,
    ) -> Result<impl Future<Output = Result<FileContents, Error>> + use<B>, Error> {
        let options = options
            .map(FileOptions::from)
            .unwrap_or_default();
        let mut lease = self.descriptors.lease(self.fd)?;

        Ok(async move {
            let bytes = lease.session()?.read_to_end().await?;

            match options.encoding()? {
                Some(encoding) => Ok(FileContents::Text(encoding.decode(&bytes))),
                None => Ok(FileContents::Bytes(Bytes::from(bytes))),
            }
        })
    }

    #[guestjs(async_method)]
    fn write_file(
        &self,
        data: FileData,
        options: Option<EncodingOrOptions>,
    ) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let options = options
            .map(FileOptions::from)
            .unwrap_or_default();
        let mut lease = self.descriptors.lease(self.fd)?;

        Ok(async move {
            lease
                .session()?
                .write(data.into_bytes(options.encoding()?)?, None)
                .await?;

            Ok(())
        })
    }

    #[guestjs(async_method)]
    fn read(
        &self,
        source: Option<ReadSource<B>>,
        offset: Option<u32>,
        length: Option<u32>,
        position: Option<u64>,
    ) -> Result<impl Future<Output = Result<ReadResult<B>, Error>> + use<B>, Error> {
        let request = ReadRequest::new(source, offset, length, position)?;
        let mut lease = self.descriptors.lease(self.fd)?;

        Ok(async move {
            let bytes = lease
                .session()?
                .read(request.length, request.position)
                .await?;

            Ok(ReadResult {
                bytes_read: bytes.len() as u32,
                buffer: match request.buffer {
                    Some(buffer) => {
                        buffer.write(request.offset, &bytes)?;

                        BufferValue::Value(buffer)
                    }
                    None => BufferValue::Bytes(Bytes::from(bytes)),
                },
            })
        })
    }

    #[guestjs(async_method)]
    fn write(
        &self,
        source: WriteSource<B>,
        offset_or_position: Option<u64>,
        length_or_encoding: Option<LengthOrEncoding>,
        position: Option<u64>,
    ) -> Result<impl Future<Output = Result<WriteResult<B>, Error>> + use<B>, Error> {
        let request = WriteRequest::new(source, offset_or_position, length_or_encoding, position)?;
        let mut lease = self.descriptors.lease(self.fd)?;

        Ok(async move {
            Ok(WriteResult {
                bytes_written: lease
                    .session()?
                    .write(request.bytes, request.position)
                    .await? as u32,
                buffer: request.buffer,
            })
        })
    }
}
