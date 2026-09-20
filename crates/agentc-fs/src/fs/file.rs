// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    io::{Result as IoResult, SeekFrom},
    pin::Pin,
    task::{Context, Poll},
};

use tokio::io::{
    AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, AsyncWrite, AsyncWriteExt, ReadBuf,
};

use crate::{
    backend::FileHandle,
    errors::{Error, IntoFsError},
    path::{Path, PathBuf},
};

pub struct File {
    inner: Box<dyn FileHandle>,
    path: PathBuf,
}

impl File {
    pub(crate) fn new(inner: Box<dyn FileHandle>, path: PathBuf) -> Self {
        Self { inner, path }
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub async fn read_to_end(&mut self) -> Result<Vec<u8>, Error> {
        let mut bytes = Vec::new();
        AsyncReadExt::read_to_end(&mut *self, &mut bytes)
            .await
            .map_err(|error| error.into_fs_error(self.path.as_path(), "failed to read file"))?;

        Ok(bytes)
    }

    pub async fn read_to_string(&mut self) -> Result<String, Error> {
        let mut content = String::new();
        AsyncReadExt::read_to_string(&mut *self, &mut content)
            .await
            .map_err(|error| {
                error.into_fs_error(self.path.as_path(), "failed to read file as string")
            })?;

        Ok(content)
    }

    pub async fn write_all(&mut self, bytes: impl AsRef<[u8]>) -> Result<(), Error> {
        AsyncWriteExt::write_all(&mut *self, bytes.as_ref())
            .await
            .map_err(|error| error.into_fs_error(self.path.as_path(), "failed to write file"))
    }

    pub async fn flush(&mut self) -> Result<(), Error> {
        AsyncWriteExt::flush(&mut *self)
            .await
            .map_err(|error| error.into_fs_error(self.path.as_path(), "failed to flush file"))
    }

    pub async fn read(&mut self, len: u64) -> Result<Vec<u8>, Error> {
        let mut bytes = Vec::new();
        AsyncReadExt::read_to_end(&mut AsyncReadExt::take(&mut *self, len), &mut bytes)
            .await
            .map_err(|error| error.into_fs_error(self.path.as_path(), "failed to read file"))?;

        Ok(bytes)
    }

    pub async fn seek(&mut self, position: SeekFrom) -> Result<u64, Error> {
        AsyncSeekExt::seek(&mut *self, position)
            .await
            .map_err(|error| error.into_fs_error(self.path.as_path(), "failed to seek file"))
    }

    pub async fn stream_position(&mut self) -> Result<u64, Error> {
        AsyncSeekExt::stream_position(&mut *self)
            .await
            .map_err(|error| {
                error.into_fs_error(self.path.as_path(), "failed to get file position")
            })
    }

    pub async fn set_len(&mut self, len: u64) -> Result<(), Error> {
        self.inner.set_len(len).await
    }

    pub async fn sync_all(&mut self) -> Result<(), Error> {
        self.inner.sync_all().await
    }

    pub async fn sync_data(&mut self) -> Result<(), Error> {
        self.inner.sync_data().await
    }
}

impl AsyncRead for File {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        self.get_mut().inner.poll_read(cx, buf)
    }
}

impl AsyncWrite for File {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<IoResult<usize>> {
        self.get_mut().inner.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        self.get_mut().inner.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        self.get_mut().inner.poll_shutdown(cx)
    }
}

impl AsyncSeek for File {
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> IoResult<()> {
        self.get_mut()
            .inner
            .start_seek(position)
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<u64>> {
        self.get_mut().inner.poll_seek(cx)
    }
}

#[cfg(test)]
mod tests {
    use crate::fs::Fs;

    #[tokio::test]
    async fn file_sync_operations_succeed_on_a_memory_backend() {
        let mut file = Fs::memory()
            .root()
            .options()
            .write(true)
            .create(true)
            .open("/notes.txt")
            .await
            .unwrap();

        file.write_all(b"content")
            .await
            .unwrap();

        assert!(file.sync_all().await.is_ok());
        assert!(file.sync_data().await.is_ok());
    }
}
