// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    cmp::min,
    io::{Error as IoError, ErrorKind, Result as IoResult, SeekFrom},
    task::{Context, Poll},
};

use async_trait::async_trait;
use tokio::io::ReadBuf;

use crate::{backend::FileHandle, errors::Error, path::PathBuf};

pub struct EmbeddedFile {
    path: PathBuf,
    bytes: &'static [u8],
    position: u64,
}

impl EmbeddedFile {
    pub(crate) fn new(path: PathBuf, bytes: &'static [u8]) -> Self {
        EmbeddedFile { path, bytes, position: 0 }
    }
}

#[async_trait]
impl FileHandle for EmbeddedFile {
    fn poll_read(&mut self, _cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<IoResult<()>> {
        let available = self
            .bytes
            .get(self.position as usize..)
            .unwrap_or_default();
        let len = min(available.len(), buf.remaining());

        buf.put_slice(&available[..len]);
        self.position += len as u64;

        Poll::Ready(Ok(()))
    }

    fn poll_write(&mut self, _cx: &mut Context<'_>, _bytes: &[u8]) -> Poll<IoResult<usize>> {
        Poll::Ready(Err(IoError::new(ErrorKind::PermissionDenied, "embedded files are read-only")))
    }

    fn poll_flush(&mut self, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(&mut self, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Ok(()))
    }

    fn start_seek(&mut self, position: SeekFrom) -> IoResult<()> {
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

    fn poll_seek(&mut self, _cx: &mut Context<'_>) -> Poll<IoResult<u64>> {
        Poll::Ready(Ok(self.position))
    }

    async fn set_len(&mut self, _len: u64) -> Result<(), Error> {
        Err(Error::permission_denied(self.path.clone()))
    }

    async fn sync_all(&mut self) -> Result<(), Error> {
        Ok(())
    }

    async fn sync_data(&mut self) -> Result<(), Error> {
        Ok(())
    }
}
