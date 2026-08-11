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
    policy::{
        context::{
            AccessContext, EntriesContext, MetadataContext, OpenContext, RemoveContext,
            RenameContext, SymlinkContext, WriteContext,
        },
        traits::Policy,
    },
};

pub struct PolicyFs {
    inner: Arc<dyn ErasedBackend>,
    policies: Arc<[Arc<dyn Policy>]>,
}

impl PolicyFs {
    pub fn new(backend: impl Backend) -> Self {
        PolicyFs {
            inner: Arc::new(backend),
            policies: Vec::new().into(),
        }
    }

    pub(crate) fn from_policies(backend: impl Backend, policies: Vec<Arc<dyn Policy>>) -> Self {
        PolicyFs {
            inner: Arc::new(backend),
            policies: Arc::from(policies),
        }
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

    fn mutates_open(options: &OpenOptions) -> bool {
        options.is_write()
            || options.is_append()
            || options.is_truncate()
            || options.is_create()
            || options.is_create_new()
    }

    pub fn with_policy(mut self, policy: impl Policy) -> Self {
        self.policies = self
            .policies
            .iter()
            .cloned()
            .chain([Arc::new(policy) as Arc<dyn Policy>])
            .collect::<Vec<_>>()
            .into();
        self
    }
}

#[async_trait]
impl Backend for PolicyFs {
    type File = Box<dyn FileHandle>;
    type DirEntries = Box<dyn DirectoryCursor>;

    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }

    async fn open(&self, path: &Path, options: &OpenOptions) -> Result<Self::File, Error> {
        self.check_open(path, options)?;

        if Self::mutates_open(options) {
            self.check_write(&WriteContext::new(path, Some(options), None, None, None))?;
        }

        self.inner.open(path, options).await
    }

    async fn entries(&self, path: &Path) -> Result<Self::DirEntries, Error> {
        self.check_entries(path)?;

        self.inner.entries(path).await
    }

    async fn metadata(&self, path: &Path, options: &MetadataOptions) -> Result<Metadata, Error> {
        self.check_metadata(path, options)?;

        self.inner.metadata(path, options).await
    }

    async fn access(&self, path: &Path, options: &AccessOptions) -> Result<(), Error> {
        self.check_access(path, options)?;

        self.inner.access(path, options).await
    }

    async fn create_dir(&self, path: &Path, options: &CreateDirOptions) -> Result<(), Error> {
        self.check_write(&WriteContext::new(path, None, Some(options), None, None))?;

        self.inner
            .create_dir(path, options)
            .await
    }

    async fn remove_file(&self, path: &Path) -> Result<(), Error> {
        self.check_remove(&RemoveContext::new(path, None))?;

        self.inner.remove_file(path).await
    }

    async fn remove_dir(&self, path: &Path, options: &RemoveDirOptions) -> Result<(), Error> {
        self.check_remove(&RemoveContext::new(path, Some(options)))?;

        self.inner
            .remove_dir(path, options)
            .await
    }

    async fn truncate(&self, path: &Path, len: u64) -> Result<(), Error> {
        self.check_write(&WriteContext::new(path, None, None, None, None))?;

        self.inner.truncate(path, len).await
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<(), Error> {
        self.check_rename(from, to)?;

        self.inner.rename(from, to).await
    }

    async fn symlink(&self, target: &Path, link: &Path) -> Result<(), Error> {
        self.check_symlink(target, link)?;

        self.inner.symlink(target, link).await
    }

    async fn read_link(&self, path: &Path) -> Result<PathBuf, Error> {
        self.check_metadata(path, &MetadataOptions::new().follow_symlinks(false))?;

        self.inner.read_link(path).await
    }

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> Result<(), Error> {
        self.check_write(&WriteContext::new(path, None, None, Some(&permissions), None))?;

        self.inner
            .set_permissions(path, permissions)
            .await
    }

    async fn set_owner(
        &self,
        path: &Path,
        owner: Owner,
        options: &SetOwnerOptions,
    ) -> Result<(), Error> {
        self.check_write(&WriteContext::new(path, None, None, None, Some(&owner)))?;

        self.inner
            .set_owner(path, owner, options)
            .await
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
        policy::{
            AccessContext, Denied, OpenContext, Policy, PolicyFs, RenameContext, WriteContext,
        },
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
    async fn allows_operations_when_policies_allow() {
        let mut file =
            Fs::new(PolicyFs::new(MemorySource::with_file("/notes.txt", b"allowed").await))
                .root()
                .open_file("/notes.txt")
                .await
                .unwrap();

        assert_eq!(file.read_to_string().await.unwrap(), "allowed");
    }

    #[tokio::test]
    async fn denies_open_when_policy_rejects_open() {
        assert!(matches!(
            Fs::new(
                PolicyFs::new(MemorySource::with_file("/notes.txt", b"denied").await)
                    .with_policy(DenyOpen)
            )
            .root()
            .open_file("/notes.txt")
            .await,
            Err(Error::PermissionDenied(path)) if path.to_string_lossy() == "/notes.txt"
        ));
    }

    #[tokio::test]
    async fn policy_can_deny_access() {
        let root = Fs::new(
            PolicyFs::new(MemorySource::with_file("/notes.txt", b"access").await)
                .with_policy(DenyAccess),
        )
        .root();

        assert!(matches!(
            root.access("/notes.txt", &AccessOptions::new()).await,
            Err(Error::PermissionDenied(path)) if path.to_string_lossy() == "/notes.txt"
        ));

        root.metadata("/notes.txt")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn denies_mutating_open_when_policy_rejects_write() {
        assert!(matches!(
            Fs::new(PolicyFs::new(MemoryFs::new()).with_policy(DenyWrite))
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
    async fn denies_rename_when_policy_rejects_rename() {
        assert!(matches!(
            Fs::new(
                PolicyFs::new(MemorySource::with_file("/notes.txt", b"rename").await)
                    .with_policy(DenyRename)
            )
            .root()
            .rename("/notes.txt", "/renamed.txt")
            .await,
            Err(Error::PermissionDenied(path)) if path.to_string_lossy() == "/notes.txt"
        ));
    }

    #[tokio::test]
    async fn policy_can_deny_set_owner_by_inspecting_the_owner() {
        let root = Fs::new(
            PolicyFs::new(MemorySource::with_file("/notes.txt", b"owner").await)
                .with_policy(DenyOwner),
        )
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
}
