// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    io::{Result as IoResult, SeekFrom},
    pin::Pin,
    task::{Context, Poll},
};

use async_trait::async_trait;
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncSeek, AsyncWrite, ReadBuf},
};

use crate::{
    backend::FileHandle,
    errors::{Error, IntoFsError},
    path::PathBuf,
};

pub struct HostFile {
    path: PathBuf,
    file: File,
}

impl HostFile {
    pub(crate) fn new(path: PathBuf, file: File) -> Self {
        HostFile { path, file }
    }
}

#[async_trait]
impl FileHandle for HostFile {
    fn poll_read(&mut self, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut self.file).poll_read(cx, buf)
    }

    fn poll_write(&mut self, cx: &mut Context<'_>, bytes: &[u8]) -> Poll<IoResult<usize>> {
        Pin::new(&mut self.file).poll_write(cx, bytes)
    }

    fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut self.file).poll_flush(cx)
    }

    fn poll_shutdown(&mut self, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut self.file).poll_shutdown(cx)
    }

    fn start_seek(&mut self, position: SeekFrom) -> IoResult<()> {
        Pin::new(&mut self.file).start_seek(position)
    }

    fn poll_seek(&mut self, cx: &mut Context<'_>) -> Poll<IoResult<u64>> {
        Pin::new(&mut self.file).poll_complete(cx)
    }

    async fn set_len(&mut self, len: u64) -> Result<(), Error> {
        self.file
            .set_len(len)
            .await
            .map_err(|error| {
                error.into_fs_error(self.path.as_path(), "failed to resize host file")
            })
    }

    async fn sync_all(&mut self) -> Result<(), Error> {
        self.file
            .sync_all()
            .await
            .map_err(|error| {
                error.into_fs_error(self.path.as_path(), "failed to sync host file")
            })
    }

    async fn sync_data(&mut self) -> Result<(), Error> {
        self.file
            .sync_data()
            .await
            .map_err(|error| {
                error.into_fs_error(self.path.as_path(), "failed to sync host file data")
            })
    }
}
