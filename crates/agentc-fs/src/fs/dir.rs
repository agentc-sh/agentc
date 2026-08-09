// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Stream, future::poll_fn};

use crate::{
    backend::DirectoryCursor,
    errors::Error,
    fs::{
        file::File,
        filesystem::Fs,
        types::{
            CreateDirOptions, FileType, Metadata, MetadataOptions, OpenOptions, OpenOptionsBuilder,
            RemoveDirOptions,
        },
    },
    path::{Component, IntoPathBuf, Path, PathBuf},
};

#[derive(Clone)]
pub struct Dir {
    fs: Fs,
    root: PathBuf,
    path: PathBuf,
}

impl Dir {
    pub(crate) fn new(fs: Fs, root: PathBuf, path: PathBuf) -> Self {
        Dir { fs, root, path }
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub fn authority_root(&self) -> &Path {
        self.root.as_path()
    }

    pub async fn open_file(&self, path: impl IntoPathBuf) -> Result<File, Error> {
        self.open_with_options(path, &OpenOptions::new().read(true))
            .await
    }

    pub(crate) async fn open_with_options(
        &self,
        path: impl IntoPathBuf,
        options: &OpenOptions,
    ) -> Result<File, Error> {
        Ok(File::new(
            self.fs
                .backend
                .open(self.resolve(path)?.as_path(), options)
                .await?,
        ))
    }

    pub fn options(&self) -> OpenOptionsBuilder<'_> {
        OpenOptionsBuilder::new(self)
    }

    pub async fn open_dir(&self, path: impl IntoPathBuf) -> Result<Dir, Error> {
        let path = self.resolve(path)?;

        if self
            .fs
            .backend
            .metadata(path.as_path(), &MetadataOptions::new())
            .await?
            .file_type()
            != FileType::Directory
        {
            return Err(Error::not_directory(path));
        }

        Ok(Dir::new(self.fs.clone(), self.root.clone(), path))
    }

    pub async fn create_dir(&self, path: impl IntoPathBuf) -> Result<Dir, Error> {
        let path = self.resolve(path)?;

        self.fs
            .backend
            .create_dir(path.as_path(), &CreateDirOptions::new())
            .await?;

        Ok(Dir::new(self.fs.clone(), self.root.clone(), path))
    }

    pub async fn create_dir_all(&self, path: impl IntoPathBuf) -> Result<Dir, Error> {
        let path = self.resolve(path)?;

        self.fs
            .backend
            .create_dir(path.as_path(), &CreateDirOptions::new().recursive(true))
            .await?;

        Ok(Dir::new(self.fs.clone(), self.root.clone(), path))
    }

    pub async fn entries(&self) -> Result<DirEntries, Error> {
        Ok(DirEntries::new(
            self.fs
                .backend
                .entries(self.path.as_path())
                .await?,
        ))
    }

    pub async fn entry(&self, path: impl IntoPathBuf) -> Result<DirEntry, Error> {
        let path = self.resolve(path)?;
        let metadata = self
            .fs
            .backend
            .metadata(path.as_path(), &MetadataOptions::new())
            .await?;
        let file_name = path
            .file_name()
            .ok_or_else(|| Error::invalid_path("path does not have a file name"))?;

        Ok(DirEntry::new(path, file_name, metadata.file_type(), metadata))
    }

    pub async fn metadata(&self, path: impl IntoPathBuf) -> Result<Metadata, Error> {
        self.fs
            .backend
            .metadata(self.resolve(path)?.as_path(), &MetadataOptions::new())
            .await
    }

    pub async fn symlink(
        &self,
        target: impl IntoPathBuf,
        link: impl IntoPathBuf,
    ) -> Result<(), Error> {
        self.fs
            .backend
            .symlink(self.resolve(target)?.as_path(), self.resolve(link)?.as_path())
            .await
    }

    pub async fn read_link(&self, path: impl IntoPathBuf) -> Result<PathBuf, Error> {
        self.fs
            .backend
            .read_link(self.resolve(path)?.as_path())
            .await
    }

    pub async fn rename(&self, from: impl IntoPathBuf, to: impl IntoPathBuf) -> Result<(), Error> {
        self.fs
            .backend
            .rename(self.resolve(from)?.as_path(), self.resolve(to)?.as_path())
            .await
    }

    pub async fn move_path(
        &self,
        from: impl IntoPathBuf,
        to: impl IntoPathBuf,
    ) -> Result<(), Error> {
        self.rename(from, to).await
    }

    pub async fn remove_file(&self, path: impl IntoPathBuf) -> Result<(), Error> {
        self.fs
            .backend
            .remove_file(self.resolve(path)?.as_path())
            .await
    }

    pub async fn remove_dir(&self, path: impl IntoPathBuf) -> Result<(), Error> {
        self.fs
            .backend
            .remove_dir(self.resolve(path)?.as_path(), &RemoveDirOptions::new())
            .await
    }

    pub async fn remove_dir_all(&self, path: impl IntoPathBuf) -> Result<(), Error> {
        self.fs
            .backend
            .remove_dir(self.resolve(path)?.as_path(), &RemoveDirOptions::new().recursive(true))
            .await
    }

    pub(crate) fn resolve(&self, path: impl IntoPathBuf) -> Result<PathBuf, Error> {
        let path = path.into_path_buf()?;

        if path.is_absolute() && self.root != PathBuf::root() {
            return Err(Error::path_escapes_authority(path.clone()));
        }

        let mut components = if path.is_absolute() {
            Vec::new()
        } else {
            self.path_components(&self.path)
        };
        let root_len = self.path_components(&self.root).len();

        for component in path.components() {
            match component.as_bytes() {
                b"/" => {}
                b"." => {}
                b".." if components.len() > root_len => {
                    components.pop();
                }
                b".." => return Err(Error::path_escapes_authority(path.clone())),
                _ => components.push(component),
            }
        }

        PathBuf::parse(Self::path_bytes(components))
    }

    fn path_components(&self, path: &PathBuf) -> Vec<Component> {
        path.components()
            .filter(|component| !matches!(component.as_bytes(), b"/" | b"."))
            .collect()
    }

    fn path_bytes(components: Vec<Component>) -> Vec<u8> {
        let mut bytes = vec![b'/'];

        for component in components {
            if bytes.len() > 1 {
                bytes.push(b'/');
            }

            bytes.extend_from_slice(component.as_bytes());
        }

        bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirEntry {
    path: PathBuf,
    file_name: Component,
    file_type: FileType,
    metadata: Option<Metadata>,
}

impl DirEntry {
    pub fn new(
        path: impl Into<PathBuf>,
        file_name: Component,
        file_type: FileType,
        metadata: impl Into<Option<Metadata>>,
    ) -> Self {
        DirEntry {
            path: path.into(),
            file_name,
            file_type,
            metadata: metadata.into(),
        }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn file_name(&self) -> &Component {
        &self.file_name
    }

    pub fn file_type(&self) -> FileType {
        self.file_type
    }

    pub fn metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
    }
}

pub struct DirEntries {
    inner: Box<dyn DirectoryCursor>,
}

impl DirEntries {
    pub(crate) fn new(inner: Box<dyn DirectoryCursor>) -> Self {
        DirEntries { inner }
    }

    pub async fn next(&mut self) -> Result<Option<DirEntry>, Error> {
        poll_fn(|cx| {
            Pin::new(&mut *self.inner)
                .poll_next(cx)
                .map(|entry| entry.transpose())
        })
        .await
    }
}

impl Stream for DirEntries {
    type Item = Result<DirEntry, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut *self.inner).poll_next(cx)
    }
}
