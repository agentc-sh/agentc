// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    io::{Result as IoResult, SeekFrom},
    task::{Context, Poll},
};

use async_trait::async_trait;
use futures::Stream;
use tokio::io::ReadBuf;

use crate::{
    errors::Error,
    fs::{
        Capabilities, CreateDirOptions, DirEntry, Metadata, MetadataOptions, OpenOptions,
        Owner, Permissions, RemoveDirOptions, SetOwnerOptions,
    },
    path::{Path, PathBuf},
};

#[async_trait]
pub trait Backend: Send + Sync + 'static {
    type File: FileHandle;
    type DirEntries: DirectoryCursor;

    fn capabilities(&self) -> Capabilities;

    async fn open(&self, path: &Path, options: &OpenOptions) -> Result<Self::File, Error>;

    async fn entries(&self, path: &Path) -> Result<Self::DirEntries, Error>;

    async fn metadata(&self, path: &Path, options: &MetadataOptions) -> Result<Metadata, Error>;

    async fn create_dir(&self, path: &Path, options: &CreateDirOptions) -> Result<(), Error>;

    async fn remove_file(&self, path: &Path) -> Result<(), Error>;

    async fn remove_dir(&self, path: &Path, options: &RemoveDirOptions) -> Result<(), Error>;

    async fn truncate(&self, path: &Path, len: u64) -> Result<(), Error>;

    async fn rename(&self, from: &Path, to: &Path) -> Result<(), Error>;

    async fn symlink(&self, target: &Path, link: &Path) -> Result<(), Error>;

    async fn read_link(&self, path: &Path) -> Result<PathBuf, Error>;

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> Result<(), Error>;

    async fn set_owner(
        &self,
        path: &Path,
        owner: Owner,
        options: &SetOwnerOptions,
    ) -> Result<(), Error>;
}

#[async_trait]
pub trait FileHandle: Send + Unpin + 'static {
    fn poll_read(&mut self, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<IoResult<()>>;

    fn poll_write(&mut self, cx: &mut Context<'_>, bytes: &[u8]) -> Poll<IoResult<usize>>;

    fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<IoResult<()>>;

    fn poll_shutdown(&mut self, cx: &mut Context<'_>) -> Poll<IoResult<()>>;

    fn start_seek(&mut self, position: SeekFrom) -> IoResult<()>;

    fn poll_seek(&mut self, cx: &mut Context<'_>) -> Poll<IoResult<u64>>;

    async fn set_len(&mut self, len: u64) -> Result<(), Error>;

    async fn sync_all(&mut self) -> Result<(), Error>;

    async fn sync_data(&mut self) -> Result<(), Error>;
}

#[async_trait]
impl FileHandle for Box<dyn FileHandle> {
    fn poll_read(&mut self, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<IoResult<()>> {
        self.as_mut().poll_read(cx, buf)
    }

    fn poll_write(&mut self, cx: &mut Context<'_>, bytes: &[u8]) -> Poll<IoResult<usize>> {
        self.as_mut().poll_write(cx, bytes)
    }

    fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        self.as_mut().poll_flush(cx)
    }

    fn poll_shutdown(&mut self, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        self.as_mut().poll_shutdown(cx)
    }

    fn start_seek(&mut self, position: SeekFrom) -> IoResult<()> {
        self.as_mut().start_seek(position)
    }

    fn poll_seek(&mut self, cx: &mut Context<'_>) -> Poll<IoResult<u64>> {
        self.as_mut().poll_seek(cx)
    }

    async fn set_len(&mut self, len: u64) -> Result<(), Error> {
        self.as_mut().set_len(len).await
    }

    async fn sync_all(&mut self) -> Result<(), Error> {
        self.as_mut().sync_all().await
    }

    async fn sync_data(&mut self) -> Result<(), Error> {
        self.as_mut().sync_data().await
    }
}

pub trait DirectoryCursor: Stream<Item = Result<DirEntry, Error>> + Send + Unpin + 'static {}

impl<T> DirectoryCursor for T where
    T: Stream<Item = Result<DirEntry, Error>> + Send + Unpin + 'static
{
}

#[async_trait]
pub trait ErasedBackend: Send + Sync + 'static {
    fn capabilities(&self) -> Capabilities;

    async fn open(&self, path: &Path, options: &OpenOptions) -> Result<Box<dyn FileHandle>, Error>;

    async fn entries(&self, path: &Path) -> Result<Box<dyn DirectoryCursor>, Error>;

    async fn metadata(&self, path: &Path, options: &MetadataOptions) -> Result<Metadata, Error>;

    async fn create_dir(&self, path: &Path, options: &CreateDirOptions) -> Result<(), Error>;

    async fn remove_file(&self, path: &Path) -> Result<(), Error>;

    async fn remove_dir(&self, path: &Path, options: &RemoveDirOptions) -> Result<(), Error>;

    async fn truncate(&self, path: &Path, len: u64) -> Result<(), Error>;

    async fn rename(&self, from: &Path, to: &Path) -> Result<(), Error>;

    async fn symlink(&self, target: &Path, link: &Path) -> Result<(), Error>;

    async fn read_link(&self, path: &Path) -> Result<PathBuf, Error>;

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> Result<(), Error>;

    async fn set_owner(
        &self,
        path: &Path,
        owner: Owner,
        options: &SetOwnerOptions,
    ) -> Result<(), Error>;
}

#[async_trait]
impl<T> ErasedBackend for T
where
    T: Backend,
{
    fn capabilities(&self) -> Capabilities {
        Backend::capabilities(self)
    }

    async fn open(&self, path: &Path, options: &OpenOptions) -> Result<Box<dyn FileHandle>, Error> {
        Ok(Box::new(Backend::open(self, path, options).await?))
    }

    async fn entries(&self, path: &Path) -> Result<Box<dyn DirectoryCursor>, Error> {
        Ok(Box::new(Backend::entries(self, path).await?))
    }

    async fn metadata(&self, path: &Path, options: &MetadataOptions) -> Result<Metadata, Error> {
        Backend::metadata(self, path, options).await
    }

    async fn create_dir(&self, path: &Path, options: &CreateDirOptions) -> Result<(), Error> {
        Backend::create_dir(self, path, options).await
    }

    async fn remove_file(&self, path: &Path) -> Result<(), Error> {
        Backend::remove_file(self, path).await
    }

    async fn remove_dir(&self, path: &Path, options: &RemoveDirOptions) -> Result<(), Error> {
        Backend::remove_dir(self, path, options).await
    }

    async fn truncate(&self, path: &Path, len: u64) -> Result<(), Error> {
        Backend::truncate(self, path, len).await
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<(), Error> {
        Backend::rename(self, from, to).await
    }

    async fn symlink(&self, target: &Path, link: &Path) -> Result<(), Error> {
        Backend::symlink(self, target, link).await
    }

    async fn read_link(&self, path: &Path) -> Result<PathBuf, Error> {
        Backend::read_link(self, path).await
    }

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> Result<(), Error> {
        Backend::set_permissions(self, path, permissions).await
    }

    async fn set_owner(
        &self,
        path: &Path,
        owner: Owner,
        options: &SetOwnerOptions,
    ) -> Result<(), Error> {
        Backend::set_owner(self, path, owner, options).await
    }
}
