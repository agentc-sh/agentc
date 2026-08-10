// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;

use crate::{
    backend::{Backend, DirectoryCursor, ErasedBackend, FileHandle},
    errors::Error,
    fs::{
        AccessOptions, Capabilities, CreateDirOptions, DirEntry, Metadata, MetadataOptions,
        OpenOptions, Owner, Permissions, RemoveDirOptions, SetOwnerOptions,
    },
    path::{Path, PathBuf},
};

pub struct MountFs {
    mounts: Vec<Mount>,
}

impl MountFs {
    pub(crate) fn new(mut mounts: Vec<Mount>) -> Self {
        mounts.sort_by(|left, right| {
            right
                .path
                .as_bytes()
                .len()
                .cmp(&left.path.as_bytes().len())
        });

        for (index, mount) in mounts.iter_mut().enumerate() {
            mount.dev = index as u64 + 1;
        }

        MountFs { mounts }
    }

    fn route(&self, path: &Path) -> Result<Route<'_>, Error> {
        if self
            .mounts
            .iter()
            .any(|mount| mount.kind == MountKind::File && Self::is_descendant(&mount.path, path))
        {
            return Err(Error::not_directory(path));
        }

        self.mounts
            .iter()
            .find(|mount| mount.matches(path))
            .map(|mount| Ok(Route { mount, path: mount.local_path(path)? }))
            .transpose()?
            .ok_or_else(|| Error::not_found(path))
    }

    fn is_descendant(parent: &PathBuf, path: &Path) -> bool {
        path.as_bytes()
            .starts_with(parent.as_bytes())
            && path
                .as_bytes()
                .get(parent.as_bytes().len())
                == Some(&b'/')
    }
}

pub(crate) struct Mount {
    pub(crate) path: PathBuf,
    pub(crate) kind: MountKind,
    pub(crate) backend: Arc<dyn ErasedBackend>,
    pub(crate) dev: u64,
}

impl Mount {
    fn matches(&self, path: &Path) -> bool {
        match self.kind {
            MountKind::Directory if self.path == PathBuf::root() => true,
            MountKind::Directory => {
                path.as_bytes() == self.path.as_bytes() || MountFs::is_descendant(&self.path, path)
            }
            MountKind::File => path.as_bytes() == self.path.as_bytes(),
        }
    }

    fn local_path(&self, path: &Path) -> Result<PathBuf, Error> {
        match self.kind {
            MountKind::File => PathBuf::parse("/"),
            MountKind::Directory if self.path == PathBuf::root() => PathBuf::parse(path.as_bytes()),
            MountKind::Directory if path.as_bytes() == self.path.as_bytes() => PathBuf::parse("/"),
            MountKind::Directory => PathBuf::parse(&path.as_bytes()[self.path.as_bytes().len()..]),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum MountKind {
    Directory,
    File,
}

struct Route<'a> {
    mount: &'a Mount,
    path: PathBuf,
}

impl Route<'_> {
    async fn open(self, options: &OpenOptions) -> Result<Box<dyn FileHandle>, Error> {
        self.mount
            .backend
            .open(self.path.as_path(), options)
            .await
    }

    async fn entries(self) -> Result<Box<dyn DirectoryCursor>, Error> {
        let dev = self.mount.dev;

        Ok(Box::new(
            self.mount
                .backend
                .entries(self.path.as_path())
                .await?
                .map(move |entry| {
                    entry.map(|entry| {
                        DirEntry::new(
                            entry.path().clone(),
                            entry.file_name().clone(),
                            entry.file_type(),
                            entry
                                .metadata()
                                .cloned()
                                .map(|metadata| metadata.with_dev(dev)),
                        )
                    })
                }),
        ))
    }

    async fn metadata(self, options: &MetadataOptions) -> Result<Metadata, Error> {
        Ok(self
            .mount
            .backend
            .metadata(self.path.as_path(), options)
            .await?
            .with_dev(self.mount.dev))
    }

    async fn access(self, options: &AccessOptions) -> Result<(), Error> {
        self.mount
            .backend
            .access(self.path.as_path(), options)
            .await
    }

    async fn create_dir(self, options: &CreateDirOptions) -> Result<(), Error> {
        self.mount
            .backend
            .create_dir(self.path.as_path(), options)
            .await
    }

    async fn remove_file(self) -> Result<(), Error> {
        self.mount
            .backend
            .remove_file(self.path.as_path())
            .await
    }

    async fn remove_dir(self, options: &RemoveDirOptions) -> Result<(), Error> {
        self.mount
            .backend
            .remove_dir(self.path.as_path(), options)
            .await
    }

    async fn truncate(self, len: u64) -> Result<(), Error> {
        self.mount
            .backend
            .truncate(self.path.as_path(), len)
            .await
    }

    async fn read_link(self) -> Result<PathBuf, Error> {
        self.mount
            .backend
            .read_link(self.path.as_path())
            .await
    }

    async fn set_permissions(self, permissions: Permissions) -> Result<(), Error> {
        self.mount
            .backend
            .set_permissions(self.path.as_path(), permissions)
            .await
    }

    async fn set_owner(self, owner: Owner, options: &SetOwnerOptions) -> Result<(), Error> {
        self.mount
            .backend
            .set_owner(self.path.as_path(), owner, options)
            .await
    }
}

#[async_trait]
impl Backend for MountFs {
    type File = Box<dyn FileHandle>;
    type DirEntries = Box<dyn DirectoryCursor>;

    fn capabilities(&self) -> Capabilities {
        Capabilities::new()
    }

    async fn open(&self, path: &Path, options: &OpenOptions) -> Result<Self::File, Error> {
        self.route(path)?.open(options).await
    }

    async fn entries(&self, path: &Path) -> Result<Self::DirEntries, Error> {
        self.route(path)?.entries().await
    }

    async fn metadata(&self, path: &Path, options: &MetadataOptions) -> Result<Metadata, Error> {
        self.route(path)?
            .metadata(options)
            .await
    }

    async fn access(&self, path: &Path, options: &AccessOptions) -> Result<(), Error> {
        self.route(path)?
            .access(options)
            .await
    }

    async fn create_dir(&self, path: &Path, options: &CreateDirOptions) -> Result<(), Error> {
        self.route(path)?
            .create_dir(options)
            .await
    }

    async fn remove_file(&self, path: &Path) -> Result<(), Error> {
        self.route(path)?.remove_file().await
    }

    async fn remove_dir(&self, path: &Path, options: &RemoveDirOptions) -> Result<(), Error> {
        self.route(path)?
            .remove_dir(options)
            .await
    }

    async fn truncate(&self, path: &Path, len: u64) -> Result<(), Error> {
        self.route(path)?
            .truncate(len)
            .await
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<(), Error> {
        let from_route = self.route(from)?;
        let to_route = self.route(to)?;

        if !Arc::ptr_eq(&from_route.mount.backend, &to_route.mount.backend) {
            return Err(Error::cross_backend_rename(from, to));
        }

        from_route
            .mount
            .backend
            .rename(from_route.path.as_path(), to_route.path.as_path())
            .await
    }

    async fn symlink(&self, target: &Path, link: &Path) -> Result<(), Error> {
        let target_route = self.route(target)?;
        let link_route = self.route(link)?;

        if !Arc::ptr_eq(&target_route.mount.backend, &link_route.mount.backend) {
            return Err(Error::cross_backend_rename(target, link));
        }

        target_route
            .mount
            .backend
            .symlink(target_route.path.as_path(), link_route.path.as_path())
            .await
    }

    async fn read_link(&self, path: &Path) -> Result<PathBuf, Error> {
        self.route(path)?.read_link().await
    }

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> Result<(), Error> {
        self.route(path)?
            .set_permissions(permissions)
            .await
    }

    async fn set_owner(
        &self,
        path: &Path,
        owner: Owner,
        options: &SetOwnerOptions,
    ) -> Result<(), Error> {
        self.route(path)?
            .set_owner(owner, options)
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        backend::Backend,
        errors::Error,
        fs::{File, Fs, OpenOptions},
        memory::MemoryFs,
        path::PathBuf,
    };

    #[cfg(feature = "embedded")]
    use crate::embedded::EmbeddedFs;

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
    async fn routes_to_longest_prefix_mount() {
        let mut file = Fs::builder()
            .mount("/", MemoryFs::new())
            .mount("/skills", MemorySource::with_file("/tool.txt", b"specific").await)
            .build()
            .unwrap()
            .root()
            .open_file("/skills/tool.txt")
            .await
            .unwrap();

        assert_eq!(file.read_to_string().await.unwrap(), "specific");
    }

    #[tokio::test]
    async fn routes_directory_mount_descendants() {
        let mut file = Fs::builder()
            .mount("/workspace", MemorySource::with_file("/notes.txt", b"workspace").await)
            .build()
            .unwrap()
            .root()
            .open_file("/workspace/notes.txt")
            .await
            .unwrap();

        assert_eq!(file.read_to_string().await.unwrap(), "workspace");
    }

    #[cfg(feature = "embedded")]
    #[tokio::test]
    async fn routes_file_mount_exact_path() {
        let mut file = Fs::builder()
            .mount_file("/assets/readme.md", EmbeddedFs::file(b"asset"))
            .build()
            .unwrap()
            .root()
            .open_file("/assets/readme.md")
            .await
            .unwrap();

        assert_eq!(file.read_to_string().await.unwrap(), "asset");
    }

    #[cfg(feature = "embedded")]
    #[tokio::test]
    async fn rejects_file_mount_child_path() {
        assert!(matches!(
            Fs::builder()
                .mount_file("/assets/readme.md", EmbeddedFs::file(b"asset"))
                .build()
                .unwrap()
                .root()
                .open_file("/assets/readme.md/child")
                .await,
            Err(Error::NotDirectory(path)) if path.to_string_lossy() == "/assets/readme.md/child"
        ));
    }

    #[tokio::test]
    async fn rejects_cross_backend_rename() {
        assert!(matches!(
            Fs::builder()
                .mount("/left", MemorySource::with_file("/file.txt", b"left").await)
                .mount("/right", MemoryFs::new())
                .build()
                .unwrap()
                .root()
                .rename("/left/file.txt", "/right/file.txt")
                .await,
            Err(Error::CrossBackendRename {
                from,
                to,
            }) if from.to_string_lossy() == "/left/file.txt"
                && to.to_string_lossy() == "/right/file.txt"
        ));
    }

    #[tokio::test]
    async fn mounts_receive_distinct_device_numbers() {
        let root = Fs::builder()
            .mount("/left", MemorySource::with_file("/file.txt", b"left").await)
            .mount("/right", MemorySource::with_file("/file.txt", b"right").await)
            .build()
            .unwrap()
            .root();

        let left = root.metadata("/left/file.txt").await.unwrap().dev();
        let right = root.metadata("/right/file.txt").await.unwrap().dev();

        assert_ne!(left, 0);
        assert_ne!(right, 0);
        assert_ne!(left, right);
    }

    #[tokio::test]
    async fn mounted_directory_entries_carry_the_mount_device() {
        let root = Fs::builder()
            .mount("/workspace", MemorySource::with_file("/notes.txt", b"workspace").await)
            .build()
            .unwrap()
            .root();
        let mut entries = root.open_dir("/workspace").await.unwrap().entries().await.unwrap();

        assert_eq!(
            entries
                .next()
                .await
                .unwrap()
                .unwrap()
                .metadata()
                .unwrap()
                .dev(),
            root.metadata("/workspace/notes.txt").await.unwrap().dev()
        );
    }
}
