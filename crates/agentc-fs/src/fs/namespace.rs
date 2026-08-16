// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use arc_swap::ArcSwap;
use async_trait::async_trait;
use futures::{StreamExt, stream};

use crate::{
    backend::{Backend, DirectoryCursor, ErasedBackend, FileHandle},
    errors::Error,
    fs::{
        AccessOptions, Capabilities, CreateDirOptions, DirEntry, FileType, Metadata,
        MetadataOptions, OpenOptions, Owner, Permissions, RemoveDirOptions, SetOwnerOptions,
    },
    path::{Component, Path, PathBuf},
    policy::{
        AccessContext, EntriesContext, MetadataContext, OpenContext, Policy, RemoveContext,
        RenameContext, SymlinkContext, WriteContext,
    },
};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum MountKind {
    Directory,
    File,
}

#[derive(Clone)]
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
                path.as_bytes() == self.path.as_bytes()
                    || MountTable::is_descendant(&self.path, path)
            }
            MountKind::File => path.as_bytes() == self.path.as_bytes(),
        }
    }

    fn local_path(&self, path: &Path) -> Result<PathBuf, Error> {
        match self.kind {
            MountKind::File => PathBuf::parse("/"),
            MountKind::Directory if self.path == PathBuf::root() => {
                PathBuf::parse(path.as_bytes())
            }
            MountKind::Directory if path.as_bytes() == self.path.as_bytes() => {
                PathBuf::parse("/")
            }
            MountKind::Directory => {
                PathBuf::parse(&path.as_bytes()[self.path.as_bytes().len()..])
            }
        }
    }
}

struct MountTable {
    mounts: Vec<Mount>,
}

impl MountTable {
    fn new(mounts: Vec<Mount>) -> Self {
        let mut mounts = Self::retain_last_per_path(mounts);

        mounts.sort_by(|left, right| {
            right
                .path
                .as_bytes()
                .len()
                .cmp(&left.path.as_bytes().len())
        });

        MountTable { mounts }
    }

    fn retain_last_per_path(mounts: Vec<Mount>) -> Vec<Mount> {
        let mut retained = Vec::<Mount>::with_capacity(mounts.len());

        for mount in mounts {
            match retained
                .iter_mut()
                .find(|existing| existing.path == mount.path)
            {
                Some(existing) => *existing = mount,
                None => retained.push(mount),
            }
        }

        retained
    }

    fn is_descendant(parent: &PathBuf, path: &Path) -> bool {
        path.as_bytes().starts_with(parent.as_bytes())
            && path.as_bytes().get(parent.as_bytes().len()) == Some(&b'/')
    }

    fn child_mount_name(ancestor: &Path, mount_path: &PathBuf) -> Option<Component> {
        let mut segments = mount_path.components();

        for component in ancestor.components() {
            if segments.next()? != component {
                return None;
            }
        }

        segments.next()
    }

    fn child_mount_names(&self, path: &Path) -> Vec<Component> {
        let mut names = Vec::new();

        for mount in &self.mounts {
            if let Some(name) = Self::child_mount_name(path, &mount.path)
                && !names.contains(&name)
            {
                names.push(name);
            }
        }

        names
    }

    fn is_mount_ancestor(&self, path: &Path) -> bool {
        self.mounts
            .iter()
            .any(|mount| Self::child_mount_name(path, &mount.path).is_some())
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
            .map(|mount| {
                Ok(Route {
                    mount,
                    path: mount.local_path(path)?,
                })
            })
            .transpose()?
            .ok_or_else(|| Error::not_found(path))
    }
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
        Ok(
            self.mount
                .backend
                .metadata(self.path.as_path(), options)
                .await?
                .with_dev(self.mount.dev),
        )
    }

    async fn access(self, options: &AccessOptions) -> Result<(), Error> {
        self.mount
            .backend
            .access(self.path.as_path(), options)
            .await
    }

    async fn create_dir(self, options: &CreateDirOptions) -> Result<bool, Error> {
        self.mount
            .backend
            .create_dir(self.path.as_path(), options)
            .await
    }

    async fn remove_file(self) -> Result<(), Error> {
        self.mount.backend.remove_file(self.path.as_path()).await
    }

    async fn remove_dir(self, options: &RemoveDirOptions) -> Result<(), Error> {
        self.mount
            .backend
            .remove_dir(self.path.as_path(), options)
            .await
    }

    async fn truncate(self, len: u64) -> Result<(), Error> {
        self.mount.backend.truncate(self.path.as_path(), len).await
    }

    async fn read_link(self) -> Result<PathBuf, Error> {
        self.mount.backend.read_link(self.path.as_path()).await
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

pub(crate) struct Namespace {
    next_dev: AtomicU64,
    table: ArcSwap<MountTable>,
    policies: Arc<[Arc<dyn Policy>]>,
}

impl Namespace {
    pub(crate) fn new(mounts: Vec<Mount>, policies: Vec<Arc<dyn Policy>>) -> Self {
        let mounts = mounts
            .into_iter()
            .enumerate()
            .map(|(index, mount)| Mount {
                dev: index as u64 + 1,
                ..mount
            })
            .collect::<Vec<_>>();

        Namespace {
            next_dev: AtomicU64::new(mounts.len() as u64 + 1),
            table: ArcSwap::from_pointee(MountTable::new(mounts)),
            policies: Arc::from(policies),
        }
    }

    pub(crate) fn single_root(backend: Arc<dyn ErasedBackend>) -> Self {
        Namespace::new(
            vec![Mount {
                path: PathBuf::root(),
                kind: MountKind::Directory,
                backend,
                dev: 0,
            }],
            Vec::new(),
        )
    }

    fn snapshot(&self) -> Arc<MountTable> {
        self.table.load_full()
    }

    fn check_open(&self, path: &Path, options: &OpenOptions) -> Result<(), Error> {
        self.policies
            .iter()
            .try_for_each(|policy| {
                policy
                    .check_open(&OpenContext::new(path, options))
                    .map_err(|_| Error::permission_denied(path))
            })
    }

    fn check_access(&self, path: &Path, options: &AccessOptions) -> Result<(), Error> {
        self.policies
            .iter()
            .try_for_each(|policy| {
                policy
                    .check_access(&AccessContext::new(path, options))
                    .map_err(|_| Error::permission_denied(path))
            })
    }

    fn check_entries(&self, path: &Path) -> Result<(), Error> {
        self.policies
            .iter()
            .try_for_each(|policy| {
                policy
                    .check_entries(&EntriesContext::new(path))
                    .map_err(|_| Error::permission_denied(path))
            })
    }

    fn check_metadata(&self, path: &Path, options: &MetadataOptions) -> Result<(), Error> {
        self.policies
            .iter()
            .try_for_each(|policy| {
                policy
                    .check_metadata(&MetadataContext::new(path, options))
                    .map_err(|_| Error::permission_denied(path))
            })
    }

    fn check_write(&self, context: &WriteContext<'_>) -> Result<(), Error> {
        self.policies
            .iter()
            .try_for_each(|policy| {
                policy
                    .check_write(context)
                    .map_err(|_| Error::permission_denied(context.path()))
            })
    }

    fn check_remove(&self, context: &RemoveContext<'_>) -> Result<(), Error> {
        self.policies
            .iter()
            .try_for_each(|policy| {
                policy
                    .check_remove(context)
                    .map_err(|_| Error::permission_denied(context.path()))
            })
    }

    fn check_rename(&self, from: &Path, to: &Path) -> Result<(), Error> {
        self.policies
            .iter()
            .try_for_each(|policy| {
                policy
                    .check_rename(&RenameContext::new(from, to))
                    .map_err(|_| Error::permission_denied(from))
            })
    }

    fn check_symlink(&self, target: &Path, link: &Path) -> Result<(), Error> {
        self.policies
            .iter()
            .try_for_each(|policy| {
                policy
                    .check_symlink(&SymlinkContext::new(target, link))
                    .map_err(|_| Error::permission_denied(link))
            })
    }

    pub(crate) fn insert(&self, mount: Mount) {
        let dev = self.next_dev.fetch_add(1, Ordering::Relaxed);

        self.table.rcu(|current| {
            let mut mounts = current.mounts.clone();

            mounts.push(Mount {
                dev,
                ..mount.clone()
            });

            MountTable::new(mounts)
        });
    }
}

#[async_trait]
impl Backend for Namespace {
    type File = Box<dyn FileHandle>;
    type DirEntries = Box<dyn DirectoryCursor>;

    fn capabilities(&self) -> Capabilities {
        self.snapshot()
            .mounts
            .iter()
            .find(|mount| mount.path == PathBuf::root())
            .map(|mount| mount.backend.capabilities())
            .unwrap_or_else(Capabilities::new)
    }

    async fn open(&self, path: &Path, options: &OpenOptions) -> Result<Self::File, Error> {
        self.check_open(path, options)?;

        if options.is_write()
            || options.is_append()
            || options.is_truncate()
            || options.is_create()
            || options.is_create_new()
        {
            self.check_write(&WriteContext::new(path, Some(options), None, None, None))?;
        }

        self.snapshot().route(path)?.open(options).await
    }

    async fn entries(&self, path: &Path) -> Result<Self::DirEntries, Error> {
        self.check_entries(path)?;

        let table = self.snapshot();
        let mut entries = BTreeMap::new();
        let mut exists = false;

        match table.route(path) {
            Ok(route) => match route.entries().await {
                Ok(mut cursor) => {
                    exists = true;

                    while let Some(entry) = cursor.next().await.transpose()? {
                        entries.insert(
                            entry.file_name().as_bytes().to_vec(),
                            DirEntry::new(
                                PathBuf::from(path).join(entry.file_name().as_bytes())?,
                                entry.file_name().clone(),
                                entry.file_type(),
                                entry.metadata().cloned(),
                            ),
                        );
                    }
                }
                Err(Error::NotFound(_)) => {}
                Err(error) => return Err(error),
            },
            Err(Error::NotFound(_)) => {}
            Err(error) => return Err(error),
        }

        for name in table.child_mount_names(path) {
            exists = true;

            entries.insert(
                name.as_bytes().to_vec(),
                DirEntry::new(
                    PathBuf::from(path).join(name.as_bytes())?,
                    name,
                    FileType::Directory,
                    Metadata::directory(Permissions::new(0o555)),
                ),
            );
        }

        if !exists {
            return Err(Error::not_found(path));
        }

        Ok(Box::new(stream::iter(
            entries.into_values().map(Ok).collect::<Vec<_>>(),
        )))
    }

    async fn metadata(&self, path: &Path, options: &MetadataOptions) -> Result<Metadata, Error> {
        self.check_metadata(path, options)?;

        let table = self.snapshot();

        match table.route(path) {
            Ok(route) => match route.metadata(options).await {
                Ok(metadata) => Ok(metadata),
                Err(Error::NotFound(_)) if table.is_mount_ancestor(path) => {
                    Ok(Metadata::directory(Permissions::new(0o555)))
                }
                Err(error) => Err(error),
            },
            Err(Error::NotFound(_)) if table.is_mount_ancestor(path) => {
                Ok(Metadata::directory(Permissions::new(0o555)))
            }
            Err(error) => Err(error),
        }
    }

    async fn access(&self, path: &Path, options: &AccessOptions) -> Result<(), Error> {
        self.check_access(path, options)?;

        let table = self.snapshot();

        match table.route(path) {
            Ok(route) => match route.access(options).await {
                Err(Error::NotFound(_)) if table.is_mount_ancestor(path) => {
                    options.evaluate(path, &Metadata::directory(Permissions::new(0o555)))
                }
                other => other,
            },
            Err(Error::NotFound(_)) if table.is_mount_ancestor(path) => {
                options.evaluate(path, &Metadata::directory(Permissions::new(0o555)))
            }
            Err(error) => Err(error),
        }
    }

    async fn create_dir(&self, path: &Path, options: &CreateDirOptions) -> Result<bool, Error> {
        self.check_write(&WriteContext::new(path, None, Some(options), None, None))?;

        self.snapshot().route(path)?.create_dir(options).await
    }

    async fn remove_file(&self, path: &Path) -> Result<(), Error> {
        self.check_remove(&RemoveContext::new(path, None))?;

        self.snapshot().route(path)?.remove_file().await
    }

    async fn remove_dir(&self, path: &Path, options: &RemoveDirOptions) -> Result<(), Error> {
        self.check_remove(&RemoveContext::new(path, Some(options)))?;

        self.snapshot().route(path)?.remove_dir(options).await
    }

    async fn truncate(&self, path: &Path, len: u64) -> Result<(), Error> {
        self.check_write(&WriteContext::new(path, None, None, None, None))?;

        self.snapshot().route(path)?.truncate(len).await
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<(), Error> {
        self.check_rename(from, to)?;

        let table = self.snapshot();
        let from_route = table.route(from)?;
        let to_route = table.route(to)?;

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
        self.check_symlink(target, link)?;

        let table = self.snapshot();
        let target_route = table.route(target)?;
        let link_route = table.route(link)?;

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
        self.check_metadata(path, &MetadataOptions::new().follow_symlinks(false))?;

        self.snapshot().route(path)?.read_link().await
    }

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> Result<(), Error> {
        self.check_write(&WriteContext::new(path, None, None, Some(&permissions), None))?;

        self.snapshot()
            .route(path)?
            .set_permissions(permissions)
            .await
    }

    async fn set_owner(
        &self,
        path: &Path,
        owner: Owner,
        options: &SetOwnerOptions,
    ) -> Result<(), Error> {
        self.check_write(&WriteContext::new(path, None, None, None, Some(&owner)))?;

        self.snapshot().route(path)?.set_owner(owner, options).await
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        backend::Backend,
        errors::Error,
        fs::{AccessOptions, File, FileType, Fs, OpenOptions, Owner},
        memory::MemoryFs,
        path::PathBuf,
        policy::{
            AccessContext, Denied, OpenContext, Policy, RenameContext, WriteContext,
        },
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
                    &OpenOptions::new().write(true).create(true),
                )
                .await
                .unwrap(),
            ));

            file.write_all(content).await.unwrap();
            fs
        }
    }

    struct DenyOpen;

    impl Policy for DenyOpen {
        fn name(&self) -> &'static str {
            "deny-open"
        }

        fn check_open(&self, _context: &OpenContext<'_>) -> Result<(), Denied> {
            Err(Denied::new("open denied"))
        }
    }

    struct DenyAccess;

    impl Policy for DenyAccess {
        fn name(&self) -> &'static str {
            "deny-access"
        }

        fn check_access(&self, _context: &AccessContext<'_>) -> Result<(), Denied> {
            Err(Denied::new("access denied"))
        }
    }

    struct DenyWrite;

    impl Policy for DenyWrite {
        fn name(&self) -> &'static str {
            "deny-write"
        }

        fn check_write(&self, _context: &WriteContext<'_>) -> Result<(), Denied> {
            Err(Denied::new("write denied"))
        }
    }

    struct DenyRename;

    impl Policy for DenyRename {
        fn name(&self) -> &'static str {
            "deny-rename"
        }

        fn check_rename(&self, _context: &RenameContext<'_>) -> Result<(), Denied> {
            Err(Denied::new("rename denied"))
        }
    }

    struct DenyOwner;

    impl Policy for DenyOwner {
        fn name(&self) -> &'static str {
            "deny-owner"
        }

        fn check_write(&self, context: &WriteContext<'_>) -> Result<(), Denied> {
            if context.owner().is_some() {
                return Err(Denied::new("owner denied"));
            }

            Ok(())
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
    async fn registering_a_second_mount_at_the_same_path_replaces_the_first() {
        let mut file = Fs::builder()
            .mount("/data", MemorySource::with_file("/file.txt", b"first").await)
            .mount("/data", MemorySource::with_file("/file.txt", b"second").await)
            .build()
            .unwrap()
            .root()
            .open_file("/data/file.txt")
            .await
            .unwrap();

        assert_eq!(file.read_to_string().await.unwrap(), "second");
    }

    #[tokio::test]
    async fn routes_directory_mount_descendants() {
        let mut file = Fs::builder()
            .mount(
                "/workspace",
                MemorySource::with_file("/notes.txt", b"workspace").await,
            )
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
            .mount(
                "/workspace",
                MemorySource::with_file("/notes.txt", b"workspace").await,
            )
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

    #[tokio::test]
    async fn allows_operations_when_policies_allow() {
        let mut file = Fs::builder()
            .mount(
                "/",
                MemorySource::with_file("/notes.txt", b"allowed").await,
            )
            .build()
            .unwrap()
            .root()
            .open_file("/notes.txt")
            .await
            .unwrap();

        assert_eq!(file.read_to_string().await.unwrap(), "allowed");
    }

    #[tokio::test]
    async fn a_policy_can_deny_open_through_the_namespace() {
        assert!(matches!(
            Fs::builder()
                .mount(
                    "/",
                    MemorySource::with_file("/notes.txt", b"denied").await,
                )
                .policy(DenyOpen)
                .build()
                .unwrap()
                .root()
                .open_file("/notes.txt")
                .await,
            Err(Error::PermissionDenied(path)) if path.to_string_lossy() == "/notes.txt"
        ));
    }

    #[tokio::test]
    async fn a_policy_can_deny_access_through_the_namespace() {
        let root = Fs::builder()
            .mount(
                "/",
                MemorySource::with_file("/notes.txt", b"access").await,
            )
            .policy(DenyAccess)
            .build()
            .unwrap()
            .root();

        assert!(matches!(
            root.access("/notes.txt", &AccessOptions::new()).await,
            Err(Error::PermissionDenied(path)) if path.to_string_lossy() == "/notes.txt"
        ));

        root.metadata("/notes.txt").await.unwrap();
    }

    #[tokio::test]
    async fn a_policy_can_deny_mutating_open_through_the_namespace() {
        assert!(matches!(
            Fs::builder()
                .mount("/", MemoryFs::new())
                .policy(DenyWrite)
                .build()
                .unwrap()
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
    async fn a_policy_can_deny_rename_through_the_namespace() {
        assert!(matches!(
            Fs::builder()
                .mount(
                    "/",
                    MemorySource::with_file("/notes.txt", b"rename").await,
                )
                .policy(DenyRename)
                .build()
                .unwrap()
                .root()
                .rename("/notes.txt", "/renamed.txt")
                .await,
            Err(Error::PermissionDenied(path)) if path.to_string_lossy() == "/notes.txt"
        ));
    }

    #[tokio::test]
    async fn a_policy_can_deny_set_owner_by_inspecting_the_owner_through_the_namespace() {
        let root = Fs::builder()
            .mount(
                "/",
                MemorySource::with_file("/notes.txt", b"owner").await,
            )
            .policy(DenyOwner)
            .build()
            .unwrap()
            .root();

        assert!(matches!(
            root.set_owner("/notes.txt", Owner::new().user(1000)).await,
            Err(Error::PermissionDenied(path)) if path.to_string_lossy() == "/notes.txt"
        ));

        root.options()
            .write(true)
            .open("/notes.txt")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn listing_a_mount_parent_shows_the_mount_point() {
        let mut entries = Fs::builder()
            .mount("/", MemoryFs::new())
            .mount(
                "/workspace",
                MemorySource::with_file("/notes.txt", b"workspace").await,
            )
            .build()
            .unwrap()
            .root()
            .entries()
            .await
            .unwrap();
        let entry = entries.next().await.unwrap().unwrap();

        assert_eq!(entry.file_name().as_bytes(), b"workspace");
        assert_eq!(entry.file_type(), FileType::Directory);
        assert!(entries.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn listing_an_unbacked_ancestor_shows_child_mounts() {
        let root = Fs::builder()
            .mount(
                "/skills/pdf",
                MemorySource::with_file("/reference.md", b"reference").await,
            )
            .build()
            .unwrap()
            .root();
        let mut entries = root.open_dir("/skills").await.unwrap().entries().await.unwrap();
        let entry = entries.next().await.unwrap().unwrap();

        assert_eq!(entry.file_name().as_bytes(), b"pdf");
        assert_eq!(entry.file_type(), FileType::Directory);
        assert!(entries.next().await.unwrap().is_none());
        assert_eq!(
            root.metadata("/skills")
                .await
                .unwrap()
                .permissions()
                .mode(),
            0o555
        );
    }

    #[tokio::test]
    async fn a_mount_point_shadows_a_covering_backend_entry() {
        let mut entries = Fs::builder()
            .mount(
                "/",
                MemorySource::with_file("/skills", b"file-not-dir").await,
            )
            .mount(
                "/skills",
                MemorySource::with_file("/inner.md", b"inner").await,
            )
            .build()
            .unwrap()
            .root()
            .entries()
            .await
            .unwrap();
        let entry = entries.next().await.unwrap().unwrap();

        assert_eq!(entry.file_name().as_bytes(), b"skills");
        assert_eq!(entry.file_type(), FileType::Directory);
        assert!(entries.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn a_walk_crosses_mount_boundaries_with_full_paths() {
        let mut walk = Fs::builder()
            .mount("/", MemorySource::with_file("/top.txt", b"top").await)
            .mount(
                "/nested",
                MemorySource::with_file("/leaf.txt", b"leaf").await,
            )
            .build()
            .unwrap()
            .root()
            .walk()
            .await
            .unwrap();
        let mut paths = Vec::new();

        while let Some(entry) = walk.next().await.unwrap() {
            paths.push(entry.path().to_string_lossy());
        }

        assert!(paths.contains(&"/nested".to_owned()));
        assert!(paths.contains(&"/nested/leaf.txt".to_owned()));
    }

    #[tokio::test]
    async fn writing_to_a_synthesized_ancestor_fails() {
        assert!(matches!(
            Fs::builder()
                .mount(
                    "/skills/pdf",
                    MemorySource::with_file("/reference.md", b"reference").await,
                )
                .build()
                .unwrap()
                .root()
                .options()
                .write(true)
                .create(true)
                .open("/skills/new.txt")
                .await,
            Err(Error::NotFound(path)) if path.to_string_lossy() == "/skills/new.txt"
        ));
    }
}
