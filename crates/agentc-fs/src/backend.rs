// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use futures::Stream;
use tokio::io::{AsyncRead, AsyncSeek, AsyncWrite};

use crate::{
    errors::Error,
    fs::{
        Capabilities, CreateDirOptions, DirEntry, Metadata, MetadataOptions, OpenOptions,
        Permissions, RemoveDirOptions,
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

    async fn rename(&self, from: &Path, to: &Path) -> Result<(), Error>;

    async fn symlink(&self, target: &Path, link: &Path) -> Result<(), Error>;

    async fn read_link(&self, path: &Path) -> Result<PathBuf, Error>;

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> Result<(), Error>;
}

pub trait FileHandle: AsyncRead + AsyncWrite + AsyncSeek + Send + Unpin + 'static {}

impl<T> FileHandle for T where T: AsyncRead + AsyncWrite + AsyncSeek + Send + Unpin + 'static {}

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

    async fn rename(&self, from: &Path, to: &Path) -> Result<(), Error>;

    async fn symlink(&self, target: &Path, link: &Path) -> Result<(), Error>;

    async fn read_link(&self, path: &Path) -> Result<PathBuf, Error>;

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> Result<(), Error>;
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
}
