// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    error::Error as StdError,
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
            .map_err(|error| {
                Error::unexpected(
                    "failed to read file",
                    Some(Box::new(error) as Box<dyn StdError + Send + Sync>),
                )
            })?;

        Ok(bytes)
    }

    pub async fn read_to_string(&mut self) -> Result<String, Error> {
        let mut content = String::new();
        AsyncReadExt::read_to_string(self, &mut content)
            .await
            .map_err(|error| {
                Error::unexpected(
                    "failed to read file as string",
                    Some(Box::new(error) as Box<dyn StdError + Send + Sync>),
                )
            })?;

        Ok(content)
    }

    pub async fn write_all(&mut self, bytes: impl AsRef<[u8]>) -> Result<(), Error> {
        AsyncWriteExt::write_all(self, bytes.as_ref())
            .await
            .map_err(|error| {
                Error::unexpected(
                    "failed to write file",
                    Some(Box::new(error) as Box<dyn StdError + Send + Sync>),
                )
            })
    }

    pub async fn flush(&mut self) -> Result<(), Error> {
        AsyncWriteExt::flush(self)
            .await
            .map_err(|error| {
                Error::unexpected(
                    "failed to flush file",
                    Some(Box::new(error) as Box<dyn StdError + Send + Sync>),
                )
            })
    }
}

impl AsyncRead for File {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        Pin::new(&mut *self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for File {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<IoResult<usize>> {
        Pin::new(&mut *self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut *self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut *self.inner).poll_shutdown(cx)
    }
}

impl AsyncSeek for File {
    fn start_seek(mut self: Pin<&mut Self>, position: SeekFrom) -> IoResult<()> {
        Pin::new(&mut *self.inner).start_seek(position)
    }

    fn poll_complete(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<u64>> {
        Pin::new(&mut *self.inner).poll_complete(cx)
    }
}
