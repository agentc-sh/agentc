// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    io::{Error as IoError, ErrorKind},
    path::{Path as BashkitPath, PathBuf as BashkitPathBuf},
    time::SystemTime,
};

use agentc_fs::{
    errors::Error as FsError,
    fs::{FileType as FsFileType, Fs, Metadata as FsMetadata, Permissions},
    path::PathBuf,
};
use async_trait::async_trait;
use bashkit::{
    DirEntry as BashkitDirEntry, FileSystem, FileSystemExt, FileType as BashkitFileType,
    Metadata as BashkitMetadata,
};

trait IntoBashkitError {
    fn into_bashkit_error(self) -> bashkit::Error;
}

impl IntoBashkitError for FsError {
    fn into_bashkit_error(self) -> bashkit::Error {
        IoError::new(
            match self {
                FsError::NotFound(_) => ErrorKind::NotFound,
                FsError::AlreadyExists(_) => ErrorKind::AlreadyExists,
                FsError::NotDirectory(_) => ErrorKind::NotADirectory,
                FsError::IsDirectory(_) => ErrorKind::IsADirectory,
                FsError::PermissionDenied(_) => ErrorKind::PermissionDenied,
                FsError::PathEscapesAuthority(_) => ErrorKind::PermissionDenied,
                FsError::Unsupported { .. } => ErrorKind::Unsupported,
                FsError::InvalidPath { .. } => ErrorKind::InvalidInput,
                FsError::CrossBackendRename { .. } => ErrorKind::CrossesDevices,
                FsError::Unexpected { .. } => ErrorKind::Other,
            },
            self,
        )
        .into()
    }
}

trait IntoAgentcPath {
    fn into_agentc_path(&self) -> Result<PathBuf, bashkit::Error>;
}

impl IntoAgentcPath for BashkitPath {
    fn into_agentc_path(&self) -> Result<PathBuf, bashkit::Error> {
        PathBuf::parse(self.as_os_str().as_encoded_bytes())
            .map_err(IntoBashkitError::into_bashkit_error)
    }
}

trait IntoBashkitMetadata {
    fn into_bashkit_metadata(self) -> BashkitMetadata;
}

impl IntoBashkitMetadata for FsMetadata {
    fn into_bashkit_metadata(self) -> BashkitMetadata {
        BashkitMetadata {
            file_type: match self.file_type() {
                FsFileType::Directory => BashkitFileType::Directory,
                FsFileType::Symlink => BashkitFileType::Symlink,
                FsFileType::Fifo => BashkitFileType::Fifo,
                _ => BashkitFileType::File,
            },
            size: self.len(),
            mode: self.permissions().mode(),
            modified: self.modified().unwrap_or_else(SystemTime::now),
            created: self.created().unwrap_or_else(SystemTime::now),
        }
    }
}

pub struct BashkitFs {
    fs: Fs,
}

impl BashkitFs {
    pub fn new(fs: Fs) -> Self {
        BashkitFs { fs }
    }
}

#[async_trait]
impl FileSystemExt for BashkitFs {}

#[async_trait]
impl FileSystem for BashkitFs {
    async fn read_file(&self, path: &BashkitPath) -> bashkit::Result<Vec<u8>> {
        self.fs
            .root()
            .open_file(path.into_agentc_path()?)
            .await
            .map_err(IntoBashkitError::into_bashkit_error)?
            .read_to_end()
            .await
            .map_err(IntoBashkitError::into_bashkit_error)
    }

    async fn write_file(&self, path: &BashkitPath, content: &[u8]) -> bashkit::Result<()> {
        let mut file = self
            .fs
            .root()
            .options()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path.into_agentc_path()?)
            .await
            .map_err(IntoBashkitError::into_bashkit_error)?;

        file.write_all(content)
            .await
            .map_err(IntoBashkitError::into_bashkit_error)?;
        file.flush()
            .await
            .map_err(IntoBashkitError::into_bashkit_error)
    }

    async fn append_file(&self, path: &BashkitPath, content: &[u8]) -> bashkit::Result<()> {
        let mut file = self
            .fs
            .root()
            .options()
            .write(true)
            .create(true)
            .append(true)
            .open(path.into_agentc_path()?)
            .await
            .map_err(IntoBashkitError::into_bashkit_error)?;

        file.write_all(content)
            .await
            .map_err(IntoBashkitError::into_bashkit_error)?;
        file.flush()
            .await
            .map_err(IntoBashkitError::into_bashkit_error)
    }

    async fn mkdir(&self, path: &BashkitPath, recursive: bool) -> bashkit::Result<()> {
        if recursive {
            self.fs
                .root()
                .create_dir_all(path.into_agentc_path()?)
                .await
                .map(|_| ())
                .map_err(IntoBashkitError::into_bashkit_error)
        } else {
            match self.fs.root().create_dir(path.into_agentc_path()?).await {
                Ok((_, true)) => Ok(()),
                Ok((_, false)) => {
                    Err(IoError::new(ErrorKind::AlreadyExists, "path already exists").into())
                }
                Err(error) => Err(error.into_bashkit_error()),
            }
        }
    }

    async fn remove(&self, path: &BashkitPath, recursive: bool) -> bashkit::Result<()> {
        let path = path.into_agentc_path()?;

        if self
            .fs
            .root()
            .symlink_metadata(path.clone())
            .await
            .map_err(IntoBashkitError::into_bashkit_error)?
            .file_type()
            == FsFileType::Directory
        {
            if recursive {
                self.fs.root().remove_dir_all(path).await
            } else {
                self.fs.root().remove_dir(path).await
            }
        } else {
            self.fs.root().remove_file(path).await
        }
        .map_err(IntoBashkitError::into_bashkit_error)
    }

    async fn stat(&self, path: &BashkitPath) -> bashkit::Result<BashkitMetadata> {
        self.fs
            .root()
            .metadata(path.into_agentc_path()?)
            .await
            .map(IntoBashkitMetadata::into_bashkit_metadata)
            .map_err(IntoBashkitError::into_bashkit_error)
    }

    async fn read_dir(&self, path: &BashkitPath) -> bashkit::Result<Vec<BashkitDirEntry>> {
        let mut entries = self
            .fs
            .root()
            .open_dir(path.into_agentc_path()?)
            .await
            .map_err(IntoBashkitError::into_bashkit_error)?
            .entries()
            .await
            .map_err(IntoBashkitError::into_bashkit_error)?;
        let mut result = Vec::new();

        while let Some(entry) = entries
            .next()
            .await
            .map_err(IntoBashkitError::into_bashkit_error)?
        {
            result.push(BashkitDirEntry {
                name: entry.file_name().to_string_lossy(),
                metadata: entry
                    .metadata()
                    .cloned()
                    .ok_or_else(|| IoError::other("directory entry metadata is unavailable"))?
                    .into_bashkit_metadata(),
            });
        }

        Ok(result)
    }

    async fn exists(&self, path: &BashkitPath) -> bashkit::Result<bool> {
        match self.fs.root().metadata(path.into_agentc_path()?).await {
            Ok(_) => Ok(true),
            Err(FsError::NotFound(_)) => Ok(false),
            Err(error) => Err(error.into_bashkit_error()),
        }
    }

    async fn rename(&self, from: &BashkitPath, to: &BashkitPath) -> bashkit::Result<()> {
        self.fs
            .root()
            .rename(from.into_agentc_path()?, to.into_agentc_path()?)
            .await
            .map_err(IntoBashkitError::into_bashkit_error)
    }

    async fn copy(&self, from: &BashkitPath, to: &BashkitPath) -> bashkit::Result<()> {
        self.write_file(to, &self.read_file(from).await?).await
    }

    async fn symlink(&self, target: &BashkitPath, link: &BashkitPath) -> bashkit::Result<()> {
        self.fs
            .root()
            .symlink(target.into_agentc_path()?, link.into_agentc_path()?)
            .await
            .map_err(IntoBashkitError::into_bashkit_error)
    }

    async fn read_link(&self, path: &BashkitPath) -> bashkit::Result<BashkitPathBuf> {
        Ok(BashkitPathBuf::from(
            self.fs
                .root()
                .read_link(path.into_agentc_path()?)
                .await
                .map_err(IntoBashkitError::into_bashkit_error)?
                .to_string_lossy(),
        ))
    }

    async fn chmod(&self, path: &BashkitPath, mode: u32) -> bashkit::Result<()> {
        self.fs
            .root()
            .set_permissions(path.into_agentc_path()?, Permissions::new(mode))
            .await
            .map_err(IntoBashkitError::into_bashkit_error)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use agentc_fs::fs::Fs;
    use bashkit::FileSystem;

    use crate::bash::fs::BashkitFs;

    #[tokio::test]
    async fn reads_and_writes_the_injected_filesystem() {
        let fs = Fs::memory();
        let adapter = BashkitFs::new(fs.clone());

        adapter
            .write_file(Path::new("/notes.txt"), b"first")
            .await
            .unwrap();
        adapter
            .append_file(Path::new("/notes.txt"), b" second")
            .await
            .unwrap();

        assert_eq!(
            fs.root()
                .open_file("/notes.txt")
                .await
                .unwrap()
                .read_to_end()
                .await
                .unwrap(),
            b"first second"
        );
    }

    #[tokio::test]
    async fn delegates_directory_metadata_and_permission_operations() {
        let adapter = BashkitFs::new(Fs::memory());

        adapter.mkdir(Path::new("/one/two"), true).await.unwrap();
        adapter
            .write_file(Path::new("/one/two/file.txt"), b"content")
            .await
            .unwrap();
        adapter
            .chmod(Path::new("/one/two/file.txt"), 0o600)
            .await
            .unwrap();

        assert_eq!(
            adapter.read_dir(Path::new("/one/two")).await.unwrap().len(),
            1
        );
        assert_eq!(
            adapter.stat(Path::new("/one/two/file.txt")).await.unwrap().mode,
            0o600
        );
    }

    #[tokio::test]
    async fn delegates_copy_rename_link_and_remove_operations() {
        let adapter = BashkitFs::new(Fs::memory());

        adapter.write_file(Path::new("/a"), b"content").await.unwrap();
        adapter.copy(Path::new("/a"), Path::new("/b")).await.unwrap();
        adapter.rename(Path::new("/b"), Path::new("/c")).await.unwrap();
        adapter
            .symlink(Path::new("/c"), Path::new("/link"))
            .await
            .unwrap();

        assert_eq!(adapter.read_file(Path::new("/c")).await.unwrap(), b"content");
        assert_eq!(
            adapter.read_link(Path::new("/link")).await.unwrap(),
            Path::new("/c")
        );

        adapter.remove(Path::new("/c"), false).await.unwrap();

        assert!(!adapter.exists(Path::new("/c")).await.unwrap());
    }
}
