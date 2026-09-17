// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    backend::{Backend, DirectoryCursor, ErasedBackend, FileHandle},
    errors::Error,
    fs::{
        AccessOptions, Capabilities, CreateDirOptions, Metadata, MetadataOptions, OpenOptions,
        Owner, Permissions, RemoveDirOptions, SetOwnerOptions,
    },
    path::{Path, PathBuf},
};

pub struct ReadOnlyFs {
    inner: Arc<dyn ErasedBackend>,
}

impl ReadOnlyFs {
    pub fn new(backend: impl Backend) -> Self {
        ReadOnlyFs { inner: Arc::new(backend) }
    }

    fn readonly_metadata(metadata: Metadata) -> Metadata {
        let readonly = Permissions::new(metadata.permissions().mode() & !0o222);
        metadata.with_permissions(readonly)
    }
}

#[async_trait]
impl Backend for ReadOnlyFs {
    type File = Box<dyn FileHandle>;
    type DirEntries = Box<dyn DirectoryCursor>;

    fn capabilities(&self) -> Capabilities {
        self.inner
            .capabilities()
            .permissions(false)
            .owner(false)
    }

    async fn open(&self, path: &Path, options: &OpenOptions) -> Result<Self::File, Error> {
        if options.is_write()
            || options.is_append()
            || options.is_truncate()
            || options.is_create()
            || options.is_create_new()
        {
            return Err(Error::permission_denied(path));
        }

        self.inner.open(path, options).await
    }

    async fn entries(&self, path: &Path) -> Result<Self::DirEntries, Error> {
        self.inner.entries(path).await
    }

    async fn metadata(&self, path: &Path, options: &MetadataOptions) -> Result<Metadata, Error> {
        Ok(Self::readonly_metadata(
            self.inner
                .metadata(path, options)
                .await?,
        ))
    }

    async fn access(&self, path: &Path, options: &AccessOptions) -> Result<(), Error> {
        if options.is_write() {
            return Err(Error::permission_denied(path));
        }

        self.inner.access(path, options).await
    }

    async fn create_dir(&self, path: &Path, _options: &CreateDirOptions) -> Result<bool, Error> {
        Err(Error::permission_denied(path))
    }

    async fn remove_file(&self, path: &Path) -> Result<(), Error> {
        Err(Error::permission_denied(path))
    }

    async fn remove_dir(&self, path: &Path, _options: &RemoveDirOptions) -> Result<(), Error> {
        Err(Error::permission_denied(path))
    }

    async fn truncate(&self, path: &Path, _len: u64) -> Result<(), Error> {
        Err(Error::permission_denied(path))
    }

    async fn rename(&self, from: &Path, _to: &Path) -> Result<(), Error> {
        Err(Error::permission_denied(from))
    }

    async fn symlink(&self, _target: &Path, link: &Path) -> Result<(), Error> {
        Err(Error::permission_denied(link))
    }

    async fn read_link(&self, path: &Path) -> Result<PathBuf, Error> {
        self.inner.read_link(path).await
    }

    async fn set_permissions(&self, path: &Path, _permissions: Permissions) -> Result<(), Error> {
        Err(Error::permission_denied(path))
    }

    async fn set_owner(
        &self,
        path: &Path,
        _owner: Owner,
        _options: &SetOwnerOptions,
    ) -> Result<(), Error> {
        Err(Error::permission_denied(path))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        backend::Backend,
        errors::Error,
        fs::{AccessOptions, File, Fs, OpenOptions, Owner},
        memory::MemoryFs,
        path::PathBuf,
        readonly::ReadOnlyFs,
    };

    struct MemorySource;

    impl MemorySource {
        async fn with_file(path: &str, content: &[u8]) -> MemoryFs {
            let fs = MemoryFs::new();
            let path = PathBuf::parse(path).unwrap();
            let mut file = File::new(Box::new(
                Backend::open(
                    &fs,
                    path.as_path(),
                    &OpenOptions::new()
                        .write(true)
                        .create(true),
                )
                .await
                .unwrap(),
            ));

            file.write_all(content).await.unwrap();
            fs
        }
    }

    #[tokio::test]
    async fn allows_read_operations() {
        let mut file =
            Fs::new(ReadOnlyFs::new(MemorySource::with_file("/notes.txt", b"readonly").await))
                .root()
                .open_file("/notes.txt")
                .await
                .unwrap();

        assert_eq!(file.read_to_string().await.unwrap(), "readonly");
    }

    #[tokio::test]
    async fn denies_open_mutation() {
        assert!(matches!(
            Fs::new(ReadOnlyFs::new(MemoryFs::new()))
                .root()
                .options()
                .write(true)
                .create(true)
                .open("/notes.txt")
                .await,
            Err(Error::PermissionDenied(path)) if path.to_string_lossy() == "/notes.txt"
        ));
    }

    #[tokio::test]
    async fn denies_mutating_operations() {
        assert!(matches!(
            Fs::new(ReadOnlyFs::new(MemoryFs::new()))
                .root()
                .create_dir("/workspace")
                .await,
            Err(Error::PermissionDenied(path)) if path.to_string_lossy() == "/workspace"
        ));
    }

    #[tokio::test]
    async fn readonly_denies_truncate() {
        assert!(matches!(
            Fs::new(ReadOnlyFs::new(MemorySource::with_file("/notes.txt", b"readonly").await))
                .root()
                .truncate("/notes.txt", 4)
                .await,
            Err(Error::PermissionDenied(path)) if path.to_string_lossy() == "/notes.txt"
        ));
    }

    #[tokio::test]
    async fn readonly_denies_set_owner() {
        assert!(matches!(
            Fs::new(ReadOnlyFs::new(MemorySource::with_file("/notes.txt", b"readonly").await))
                .root()
                .set_owner("/notes.txt", Owner::new().user(1000))
                .await,
            Err(Error::PermissionDenied(path)) if path.to_string_lossy() == "/notes.txt"
        ));
    }

    #[tokio::test]
    async fn readonly_metadata_reports_no_write_bits() {
        let metadata =
            Fs::new(ReadOnlyFs::new(MemorySource::with_file("/notes.txt", b"readonly").await))
                .root()
                .metadata("/notes.txt")
                .await
                .unwrap();

        assert_eq!(metadata.permissions().mode(), 0o444);
        assert!(metadata.permissions().is_readonly());
    }

    #[tokio::test]
    async fn readonly_reports_no_permission_support() {
        assert!(
            !Fs::new(ReadOnlyFs::new(MemoryFs::new()))
                .namespace
                .capabilities()
                .supports_permissions()
        );
    }

    #[tokio::test]
    async fn readonly_access_denies_every_write_request() {
        assert!(matches!(
            Fs::new(ReadOnlyFs::new(MemorySource::with_file("/notes.txt", b"readonly").await))
                .root()
                .access("/notes.txt", &AccessOptions::new().write(true))
                .await,
            Err(Error::PermissionDenied(path)) if path.to_string_lossy() == "/notes.txt"
        ));
    }
}
