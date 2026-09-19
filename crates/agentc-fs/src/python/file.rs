// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    future::Future,
    io::SeekFrom,
    marker::PhantomData,
    rc::Rc,
};

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{
        errors::Error,
        handle::{Instance, Object},
        host::exception::{ExceptionClass, Raise},
        host_class,
        marshal::{
            FromGuest,
            describe::{Describe, Expected},
        },
        scope::Enter,
    },
};
use bytes::Bytes;
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};

use crate::python::exceptions::UnsupportedError;

pub(crate) enum Whence {
    Start,
    Current,
    End,
}

impl Whence {
    fn seek_from<B: ExecutorBackend>(self, offset: i64) -> Result<SeekFrom, Error> {
        match self {
            Self::Start => u64::try_from(offset)
                .map(SeekFrom::Start)
                .map_err(|_| {
                    Raise::<B>::new(ExceptionClass::builtin("ValueError"))
                        .arg(format!("negative seek position {offset}"))
                        .into()
                }),
            Self::Current => Ok(SeekFrom::Current(offset)),
            Self::End => Ok(SeekFrom::End(offset)),
        }
    }
}

impl<B: ExecutorBackend> FromGuest<B> for Whence {
    type Owned = Self;

    fn from_guest<'py>(enter: &Enter<'py, B>, value: B::Value<'py>) -> Result<Self::Owned, Error> {
        match i64::from_guest(enter, value)? {
            0 => Ok(Self::Start),
            1 => Ok(Self::Current),
            2 => Ok(Self::End),
            whence => Err(
                Raise::<B>::new(ExceptionClass::builtin("ValueError"))
                    .arg(format!("invalid whence ({whence}, should be 0, 1 or 2)"))
                    .into()
            ),
        }
    }
}

impl Describe for Whence {
    fn describe(expected: &mut Expected) {
        expected.push("int");
    }
}

struct FileHandle<B: ExecutorBackend> {
    file: Mutex<Option<crate::fs::File>>,
    backend: PhantomData<B>,
}

impl<B: ExecutorBackend> FileHandle<B> {
    fn closed(&self) -> bool {
        self.file
            .try_lock()
            .is_ok_and(|file| file.is_none())
    }

    async fn lock(&self) -> Result<MappedMutexGuard<'_, crate::fs::File>, Error> {
        MutexGuard::try_map(self.file.lock().await, Option::as_mut).map_err(|_| {
            Raise::<B>::host(UnsupportedError {
                message: "File is closed".into(),
            })
            .into()
        })
    }

    async fn close(&self) -> Result<(), Error> {
        let mut state = self.file.lock().await;

        if let Some(mut file) = state.take() {
            file.flush()
                .await
                .map_err(Raise::<B>::from)?;
        }

        Ok(())
    }
}

impl<B: ExecutorBackend> From<crate::fs::File> for FileHandle<B> {
    fn from(file: crate::fs::File) -> Self {
        Self {
            file: Mutex::new(Some(file)),
            backend: PhantomData,
        }
    }
}

pub struct File<B: ExecutorBackend> {
    handle: Rc<FileHandle<B>>,
}

#[host_class(backend = B, crate_path = agentc_executor_python::guestpy)]
impl<B: ExecutorBackend> File<B> {
    #[guestpy(get)]
    fn closed(&self) -> Result<bool, Error> {
        Ok(self.handle.closed())
    }

    #[guestpy(async_method)]
    fn read(
        &self,
        #[guestpy(positional)] size: Option<i64>,
    ) -> Result<impl Future<Output = Result<Bytes, Error>> + use<B>, Error> {
        let handle = self.handle.clone();

        Ok(async move {
            let mut file = handle.lock().await?;

            Ok(
                Bytes::from(
                    match size.and_then(|size| u64::try_from(size).ok()) {
                        Some(size) => file.read(size).await,
                        None => file.read_to_end().await,
                    }
                    .map_err(Raise::<B>::from)?
                )
            )
        })
    }

    #[guestpy(async_method)]
    fn read_text(&self) -> Result<impl Future<Output = Result<String, Error>> + use<B>, Error> {
        let handle = self.handle.clone();

        Ok(async move {
            Ok(
                handle
                    .lock()
                    .await?
                    .read_to_string()
                    .await
                    .map_err(Raise::<B>::from)?
            )
        })
    }

    #[guestpy(async_method)]
    fn write(
        &self,
        #[guestpy(positional)] data: Bytes,
    ) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let handle = self.handle.clone();

        Ok(async move {
            Ok(
                handle
                    .lock()
                    .await?
                    .write_all(data)
                    .await
                    .map_err(Raise::<B>::from)?
            )
        })
    }

    #[guestpy(async_method)]
    fn seek(
        &self,
        #[guestpy(positional)] offset: i64,
        #[guestpy(positional)] whence: Option<Whence>,
    ) -> Result<impl Future<Output = Result<u64, Error>> + use<B>, Error> {
        let handle = self.handle.clone();

        Ok(async move {
            Ok(
                handle
                    .lock()
                    .await?
                    .seek(whence.unwrap_or(Whence::Start).seek_from::<B>(offset)?)
                    .await
                    .map_err(Raise::<B>::from)?
            )
        })
    }

    #[guestpy(async_method)]
    fn tell(&self) -> Result<impl Future<Output = Result<u64, Error>> + use<B>, Error> {
        let handle = self.handle.clone();

        Ok(async move {
            Ok(
                handle
                    .lock()
                    .await?
                    .stream_position()
                    .await
                    .map_err(Raise::<B>::from)?
            )
        })
    }

    #[guestpy(async_method)]
    fn truncate(
        &self,
        #[guestpy(positional)] size: u64,
    ) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let handle = self.handle.clone();

        Ok(async move {
            Ok(
                handle
                    .lock()
                    .await?
                    .set_len(size)
                    .await
                    .map_err(Raise::<B>::from)?
            )
        })
    }

    #[guestpy(async_method)]
    fn flush(&self) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let handle = self.handle.clone();

        Ok(async move {
            Ok(
                handle
                    .lock()
                    .await?
                    .flush()
                    .await
                    .map_err(Raise::<B>::from)?
            )
        })
    }

    #[guestpy(async_method)]
    fn fsync(&self) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let handle = self.handle.clone();

        Ok(async move {
            Ok(
                handle
                    .lock()
                    .await?
                    .sync_all()
                    .await
                    .map_err(Raise::<B>::from)?
            )
        })
    }

    #[guestpy(async_method)]
    fn fdatasync(&self) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let handle = self.handle.clone();

        Ok(async move {
            Ok(
                handle
                    .lock()
                    .await?
                    .sync_data()
                    .await
                    .map_err(Raise::<B>::from)?
            )
        })
    }

    #[guestpy(async_method)]
    fn close(&self) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let handle = self.handle.clone();

        Ok(async move { handle.close().await })
    }

    #[guestpy(async_method, dunder = "__aenter__")]
    fn enter(
        &self,
        #[guestpy(this)] this: Instance<B, File<B>>,
    ) -> Result<impl Future<Output = Result<Instance<B, File<B>>, Error>> + use<B>, Error> {
        Ok(async move { Ok(this) })
    }

    #[guestpy(async_method, dunder = "__aexit__")]
    fn exit(
        &self,
        _exc_type: Object<B>,
        _exc_value: Object<B>,
        _traceback: Object<B>,
    ) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let handle = self.handle.clone();

        Ok(async move { handle.close().await })
    }
}

impl<B: ExecutorBackend> From<crate::fs::File> for File<B> {
    fn from(file: crate::fs::File) -> Self {
        Self {
            handle: Rc::new(FileHandle::from(file)),
        }
    }
}

#[cfg(test)]
mod tests {
    use agentc_executor_python::guestpy::rustpython::RustPython;

    use super::{FileHandle, Whence};
    use crate::fs::Fs;

    #[test]
    fn whence_builds_seek_positions() {
        assert_eq!(
            Whence::Start.seek_from::<RustPython>(4).unwrap(),
            std::io::SeekFrom::Start(4),
        );
        assert_eq!(
            Whence::Current.seek_from::<RustPython>(-4).unwrap(),
            std::io::SeekFrom::Current(-4),
        );
        assert_eq!(
            Whence::End.seek_from::<RustPython>(-4).unwrap(),
            std::io::SeekFrom::End(-4),
        );
        assert!(Whence::Start.seek_from::<RustPython>(-1).is_err());
    }

    #[tokio::test]
    async fn close_drops_the_file_and_is_idempotent() {
        let handle = FileHandle::<RustPython>::from(
            Fs::memory()
                .root()
                .options()
                .write(true)
                .create(true)
                .open("/notes.txt")
                .await
                .unwrap(),
        );

        assert!(!handle.closed());

        handle.close().await.unwrap();

        assert!(handle.closed());
        assert!(handle.lock().await.is_err());
        assert!(handle.close().await.is_ok());
    }
}
