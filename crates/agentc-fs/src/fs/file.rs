// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    io::{Result as IoResult, SeekFrom},
    pin::Pin,
    task::{Context, Poll},
};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncWrite, AsyncWriteExt, ReadBuf};

use crate::{backend::FileHandle, errors::Error};

pub struct File {
    inner: Box<dyn FileHandle>,
}

impl File {
    pub(crate) fn new(inner: Box<dyn FileHandle>) -> Self {
        File { inner }
    }

    pub async fn read_to_end(&mut self) -> Result<Vec<u8>, Error> {
        let mut bytes = Vec::new();
        AsyncReadExt::read_to_end(self, &mut bytes)
            .await
            .map_err(|error| Error::sourced_unexpected("failed to read file", error))?;

        Ok(bytes)
    }

    pub async fn read_to_string(&mut self) -> Result<String, Error> {
        let mut content = String::new();
        AsyncReadExt::read_to_string(self, &mut content)
            .await
            .map_err(|error| Error::sourced_unexpected("failed to read file as string", error))?;

        Ok(content)
    }

    pub async fn write_all(&mut self, bytes: impl AsRef<[u8]>) -> Result<(), Error> {
        AsyncWriteExt::write_all(self, bytes.as_ref())
            .await
            .map_err(|error| Error::sourced_unexpected("failed to write file", error))
    }

    pub async fn flush(&mut self) -> Result<(), Error> {
        AsyncWriteExt::flush(self)
            .await
            .map_err(|error| Error::sourced_unexpected("failed to flush file", error))
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
