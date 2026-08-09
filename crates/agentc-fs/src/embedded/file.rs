// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    cmp::min,
    io::{Error as IoError, ErrorKind, Result as IoResult, SeekFrom},
    pin::Pin,
    task::{Context, Poll},
};

use tokio::io::{AsyncRead, AsyncSeek, AsyncWrite, ReadBuf};

pub struct EmbeddedFile {
    bytes: &'static [u8],
    position: u64,
}

impl EmbeddedFile {
    pub(crate) fn new(bytes: &'static [u8]) -> Self {
        EmbeddedFile { bytes, position: 0 }
    }
}

impl AsyncRead for EmbeddedFile {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        let available = self
            .bytes
            .get(self.position as usize..)
            .unwrap_or_default();
        let len = min(available.len(), buf.remaining());

        buf.put_slice(&available[..len]);
        self.position += len as u64;

        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for EmbeddedFile {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _bytes: &[u8],
    ) -> Poll<IoResult<usize>> {
        Poll::Ready(Err(IoError::new(ErrorKind::PermissionDenied, "embedded files are read-only")))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncSeek for EmbeddedFile {
    fn start_seek(mut self: Pin<&mut Self>, position: SeekFrom) -> IoResult<()> {
        let position = match position {
            SeekFrom::Start(position) => position as i64,
            SeekFrom::End(offset) => self.bytes.len() as i64 + offset,
            SeekFrom::Current(offset) => self.position as i64 + offset,
        };

        if position < 0 {
            return Err(IoError::new(ErrorKind::InvalidInput, "cannot seek before start of file"));
        }

        self.position = position as u64;
        Ok(())
    }

    fn poll_complete(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<u64>> {
        Poll::Ready(Ok(self.position))
    }
}
