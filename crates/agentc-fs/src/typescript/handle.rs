// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::guestjs::{
    errors::Error,
    handle::{BoundObject, Object, Value, BoundObjectProtocol},
    host::{Args, ClassSpec, HostClass, HostObject, Deferred},
    marshal::{FromGuest, FromGuestBound, ToGuest, ToGuestBound},
    runtime::Scope,
};
use bytes::Bytes;

use crate::{
    fs::{Dir, Owner, Permissions},
    path::PathBuf,
    typescript::{
        descriptors::Descriptors,
        encoding::Encoding,
        options::FileOptions,
        stats::Stats,
        types::{BufferValue, BufferView, FileContents, FileData},
    },
};

struct ReadRequest {
    buffer: Option<Value>,
    offset: usize,
    length: usize,
    position: Option<u64>,
}

impl ReadRequest {
    fn default() -> Self {
        Self {
            buffer: None,
            offset: 0,
            length: 16_384,
            position: None,
        }
    }

    fn from_buffer<'js>(
        scope: &Scope<'js>,
        args: &Args<'js>,
        buffer: BoundObject<'js>,
    ) -> Result<Self, Error> {
        let offset = args
            .get_opt::<u32>(scope, 1)?
            .unwrap_or_default() as usize;
        let length = args
            .get_opt::<u32>(scope, 2)?
            .map(|length| length as usize)
            .unwrap_or(BufferView::checked_length(BufferView::len(&buffer)?, offset)?);

        Ok(Self {
            buffer: Some(Value::from_guest(scope, buffer.to_guest_bound(scope)?)?),
            offset,
            length,
            position: args.get_opt::<u64>(scope, 3)?,
        })
    }

    fn from_options<'js>(scope: &Scope<'js>, options: BoundObject<'js>) -> Result<Self, Error> {
        let buffer = options.get::<Option<Object>>("buffer")?;
        let offset = options
            .get::<Option<u32>>("offset")?
            .unwrap_or_default() as usize;
        let length = match &buffer {
            Some(buffer) => options
                .get::<Option<u32>>("length")?
                .map(|length| length as usize)
                .unwrap_or(BufferView::checked_length(BufferView::len(buffer)?, offset)?),
            None => options
                .get::<Option<u32>>("length")?
                .unwrap_or(16_384) as usize,
        };

        Ok(Self {
            buffer: buffer
                .map(|buffer| Value::from_guest(scope, buffer.to_guest_bound(scope)?))
                .transpose()?,
            offset,
            length,
            position: options.get::<Option<u64>>("position")?,
        })
    }

    fn from_args<'js>(scope: &Scope<'js>, args: &Args<'js>) -> Result<Self, Error> {
        let Some(first) = args.get_opt::<Value>(scope, 0)? else {
            return Ok(Self::default());
        };

        let Ok(first_object) = Object::from_guest_bound(scope, first.to_guest_bound(scope)?) else {
            return Ok(Self::default());
        };

        if BufferView::is_uint8_array(&first_object)? {
            return Self::from_buffer(scope, args, first_object);
        }

        Self::from_options(scope, first_object)
    }
}

struct WriteRequest {
    buffer: BufferValue,
    bytes: Vec<u8>,
    position: Option<u64>,
}

impl WriteRequest {
    fn from_buffer<'js>(
        scope: &Scope<'js>,
        args: &Args<'js>,
        buffer: BoundObject<'js>,
    ) -> Result<Self, Error> {
        let offset = args
            .get_opt::<u32>(scope, 1)?
            .unwrap_or_default() as usize;
        let length = args
            .get_opt::<u32>(scope, 2)?
            .map(|length| length as usize)
            .unwrap_or(BufferView::checked_length(BufferView::len(&buffer)?, offset)?);

        Ok(Self {
            bytes: BufferView::read(&buffer, offset, length)?,
            buffer: BufferValue::Value(Value::from_guest(scope, buffer.to_guest_bound(scope)?)?),
            position: args.get_opt::<u64>(scope, 3)?,
        })
    }

    fn from_string<'js>(scope: &Scope<'js>, args: &Args<'js>) -> Result<Self, Error> {
        let text = args.get_owned::<String>(scope, 0)?;
        let encoding = args
            .get_opt::<String>(scope, 2)?
            .map(|encoding| Encoding::parse(&encoding))
            .transpose()?
            .unwrap_or(Encoding::Utf8);

        Ok(Self {
            bytes: encoding.encode(&text)?,
            buffer: BufferValue::Text(text),
            position: args.get_opt::<u64>(scope, 1)?,
        })
    }

    fn from_args<'js>(scope: &Scope<'js>, args: &Args<'js>) -> Result<Self, Error> {
        if let Some(value) = args.get_opt::<Value>(scope, 0)? {
            if let Ok(buffer) = Object::from_guest_bound(scope, value.to_guest_bound(scope)?) {
                if BufferView::is_uint8_array(&buffer)? {
                    return Self::from_buffer(scope, args, buffer);
                }
            }
        }

        Self::from_string(scope, args)
    }
}

pub struct FileHandle {
    fd: u32,
    path: PathBuf,
    dir: Dir,
    descriptors: Descriptors,
}

impl FileHandle {
    pub fn new(fd: u32, path: PathBuf, dir: Dir, descriptors: Descriptors) -> Self {
        Self { fd, path, dir, descriptors }
    }
}

impl HostClass for FileHandle {
    const NAME: &'static str = "FileHandle";

    fn build(spec: &mut ClassSpec<Self>) {
        spec.getter("fd", |handle, _scope| Ok(handle.fd));

        spec.async_method("close", |handle, _scope, _args| {
            let descriptors = handle.descriptors.clone();
            let fd = handle.fd;

            Ok(async move {
                descriptors.close(fd)?;

                Ok(())
            })
        });

        spec.async_method("stat", |handle, _scope, _args| {
            let dir = handle.dir.clone();
            let path = handle.path.clone();

            Ok(async move { Ok(Stats::new(dir.metadata(path).await?)) })
        });

        spec.async_method("chmod", |handle, scope, args| {
            let dir = handle.dir.clone();
            let path = handle.path.clone();
            let mode = args.get::<u32>(scope, 0)?;

            Ok(async move {
                dir.set_permissions(path, Permissions::new(mode))
                    .await?;

                Ok(())
            })
        });

        spec.async_method("chown", |handle, scope, args| {
            let dir = handle.dir.clone();
            let path = handle.path.clone();
            let uid = args.get::<u32>(scope, 0)?;
            let gid = args.get::<u32>(scope, 1)?;

            Ok(async move {
                dir.set_owner(path, Owner::new().user(uid).group(gid))
                    .await?;

                Ok(())
            })
        });

        spec.async_method("truncate", |handle, scope, args| {
            let descriptors = handle.descriptors.clone();
            let fd = handle.fd;
            let len = args
                .get_opt::<u64>(scope, 0)?
                .unwrap_or_default();
            let mut lease = descriptors.lease(fd)?;

            Ok(async move {
                lease
                    .session()?
                    .truncate(len)
                    .await
                    .map_err(Into::into)
            })
        });

        spec.async_method("sync", |handle, _scope, _args| {
            let descriptors = handle.descriptors.clone();
            let fd = handle.fd;
            let mut lease = descriptors.lease(fd)?;

            Ok(async move {
                lease
                    .session()?
                    .sync_all()
                    .await
                    .map_err(Into::into)
            })
        });

        spec.async_method("datasync", |handle, _scope, _args| {
            let descriptors = handle.descriptors.clone();
            let fd = handle.fd;
            let mut lease = descriptors.lease(fd)?;

            Ok(async move {
                lease
                    .session()?
                    .sync_data()
                    .await
                    .map_err(Into::into)
            })
        });

        spec.async_method("readFile", |handle, scope, args| {
            let descriptors = handle.descriptors.clone();
            let fd = handle.fd;
            let options = FileOptions::from_args(scope, &args, 0)?;
            let mut lease = descriptors.lease(fd)?;

            Ok(async move {
                let bytes = lease.session()?.read_to_end().await?;

                match options.encoding()? {
                    Some(encoding) => Ok(FileContents::Text(encoding.decode(&bytes))),
                    None => Ok(FileContents::Bytes(Bytes::from(bytes))),
                }
            })
        });

        spec.async_method("writeFile", |handle, scope, args| {
            let descriptors = handle.descriptors.clone();
            let fd = handle.fd;
            let data = FileData::from_args(scope, &args, 0)?;
            let options = FileOptions::from_args(scope, &args, 1)?;
            let mut lease = descriptors.lease(fd)?;

            Ok(async move {
                lease
                    .session()?
                    .write(data.into_bytes(options.encoding()?)?, None)
                    .await?;

                Ok(())
            })
        });

        spec.async_method("read", |handle, scope, args| {
            let descriptors = handle.descriptors.clone();
            let fd = handle.fd;
            let request = ReadRequest::from_args(scope, &args)?;
            let mut lease = descriptors.lease(fd)?;

            Ok(async move {
                let bytes = lease
                    .session()?
                    .read(request.length, request.position)
                    .await?;

                Ok(Deferred::new(move |scope| {
                    let buffer = match request.buffer {
                        Some(buffer) => {
                            let object = buffer.bind::<Object>(scope)?;

                            BufferView::write(&object, request.offset, &bytes)?;

                            BufferValue::Value(buffer)
                        }
                        None => BufferValue::Bytes(Bytes::from(bytes.clone())),
                    };

                    Ok(Value::from_guest(
                        scope,
                        HostObject::build(|namespace| {
                            namespace.constant("bytesRead", bytes.len() as u32);
                            namespace.constant("buffer", buffer);
                        })
                        .to_guest(scope)?,
                    )?)
                }))
            })
        });

        spec.async_method("write", |handle, scope, args| {
            let descriptors = handle.descriptors.clone();
            let fd = handle.fd;
            let request = WriteRequest::from_args(scope, &args)?;
            let mut lease = descriptors.lease(fd)?;

            Ok(async move {
                let bytes_written = lease
                    .session()?
                    .write(request.bytes, request.position)
                    .await?;

                Ok(Deferred::new(move |scope| {
                    Ok(Value::from_guest(
                        scope,
                        HostObject::build(|namespace| {
                            namespace.constant("bytesWritten", bytes_written as u32);
                            namespace.constant("buffer", request.buffer.clone());
                        })
                        .to_guest(scope)?,
                    )?)
                }))
            })
        });
    }
}
