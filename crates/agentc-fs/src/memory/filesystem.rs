// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    vec::IntoIter,
};

use async_trait::async_trait;
use futures::{
    future::BoxFuture,
    stream::{self, Iter},
};
use tokio::sync::RwLock;

use crate::{
    backend::Backend,
    errors::Error,
    fs::{
        AccessOptions, Capabilities, CreateDirOptions, DirEntry, Metadata, MetadataOptions,
        OpenOptions, Owner, Permissions, RemoveDirOptions, SetOwnerOptions,
    },
    memory::{
        file::MemoryFile,
        node::{Node, NodeRef},
    },
    path::{Component, Path, PathBuf},
};

pub struct MemoryFs {
    root: NodeRef,
    next_ino: AtomicU64,
}

impl MemoryFs {
    pub fn new() -> Self {
        MemoryFs {
            root: Arc::new(RwLock::new(Node::directory(1))),
            next_ino: AtomicU64::new(2),
        }
    }

    fn next_ino(&self) -> u64 {
        self.next_ino
            .fetch_add(1, Ordering::Relaxed)
    }

    fn node<'a>(
        &'a self,
        path: &'a Path,
        follow_final_symlink: bool,
    ) -> BoxFuture<'a, Result<NodeRef, Error>> {
        Box::pin(async move {
            let mut current = self.root.clone();
            let components = self.components(path);

            for (index, component) in components.iter().enumerate() {
                let next = match &*current.read().await {
                    Node::Directory(directory) => directory
                        .entries()
                        .get(component.as_bytes())
                        .cloned()
                        .ok_or_else(|| Error::not_found(path))?,
                    _ => return Err(Error::not_directory(path)),
                };

                if follow_final_symlink || index + 1 < components.len() {
                    if let Some(target) = {
                        match &*next.read().await {
                            Node::Symlink(symlink) => Some(symlink.target()),
                            _ => None,
                        }
                    } {
                        current = self
                            .node(target.as_path(), true)
                            .await?;

                        continue;
                    }
                }

                current = next;
            }

            Ok(current)
        })
    }

    async fn parent(&self, path: &Path) -> Result<(NodeRef, Component), Error> {
        Ok((
            self.node(
                path.parent()
                    .ok_or_else(|| Error::invalid_path("path does not have a parent"))?,
                true,
            )
            .await?,
            path.file_name()
                .ok_or_else(|| Error::invalid_path("path does not have a file name"))?,
        ))
    }

    async fn child(
        &self,
        parent: &NodeRef,
        path: &Path,
        file_name: &Component,
    ) -> Result<NodeRef, Error> {
        match &*parent.read().await {
            Node::Directory(directory) => directory
                .entries()
                .get(file_name.as_bytes())
                .cloned()
                .ok_or_else(|| Error::not_found(path)),
            _ => Err(Error::not_directory(path)),
        }
    }

    fn components(&self, path: &Path) -> Vec<Component> {
        path.components()
            .filter(|component| !matches!(component.as_bytes(), b"/" | b"."))
            .collect()
    }

    fn child_path(&self, parent: &Path, file_name: &Component) -> Result<PathBuf, Error> {
        PathBuf::parse(parent.as_bytes())?.join(file_name.as_bytes())
    }
}

impl Default for MemoryFs {
    fn default() -> Self {
        MemoryFs::new()
    }
}

#[async_trait]
impl Backend for MemoryFs {
    type File = MemoryFile;
    type DirEntries = Iter<IntoIter<Result<DirEntry, Error>>>;

    fn capabilities(&self) -> Capabilities {
        Capabilities::new()
            .symlink(true)
            .atomic_rename(true)
            .permissions(true)
            .owner(true)
            .timestamps(true)
    }

    async fn open(&self, path: &Path, options: &OpenOptions) -> Result<Self::File, Error> {
        match self
            .node(path, options.follows_symlinks())
            .await
        {
            Ok(node) => {
                let content = match &mut *node.write().await {
                    Node::File(file) if options.is_create_new() => {
                        return Err(Error::already_exists(path));
                    }
                    Node::File(file) => {
                        let content = file.content();

                        if options.is_truncate() {
                            content
                                .lock()
                                .map_err(|_| Error::unexpected("memory file lock is poisoned"))?
                                .clear();
                        }

                        content
                    }
                    Node::Directory(_) => return Err(Error::is_directory(path)),
                    Node::Symlink(_) => {
                        return Err(Error::unsupported(
                            "opening symlinks without following them is unsupported",
                        ));
                    }
                };

                let position = if options.is_append() {
                    content
                        .lock()
                        .map_err(|_| Error::unexpected("memory file lock is poisoned"))?
                        .len() as u64
                } else {
                    0
                };

                Ok(MemoryFile::new(
                    PathBuf::from(path),
                    content,
                    position,
                    options.is_write() || options.is_append(),
                ))
            }
            Err(Error::NotFound(_)) if options.is_create() || options.is_create_new() => {
                let (parent, file_name) = self.parent(path).await?;

                match &mut *parent.write().await {
                    Node::Directory(directory) => {
                        if directory
                            .entries()
                            .contains_key(file_name.as_bytes())
                        {
                            return Err(Error::already_exists(path));
                        }

                        let node = Node::file(self.next_ino());
                        let content = match &node {
                            Node::File(file) => file.content(),
                            _ => unreachable!(),
                        };

                        directory
                            .entries_mut()
                            .insert(file_name.as_bytes().to_vec(), Arc::new(RwLock::new(node)));

                        Ok(MemoryFile::new(
                            PathBuf::from(path),
                            content,
                            0,
                            options.is_write() || options.is_append(),
                        ))
                    }
                    _ => Err(Error::not_directory(path)),
                }
            }
            Err(error) => Err(error),
        }
    }

    async fn entries(&self, path: &Path) -> Result<Self::DirEntries, Error> {
        let mut entries = Vec::new();

        for (file_name, child) in match &*self
            .node(path, true)
            .await?
            .read()
            .await
        {
            Node::Directory(directory) => directory
                .entries()
                .iter()
                .map(|(file_name, child)| (file_name.clone(), child.clone()))
                .collect::<Vec<_>>(),
            _ => return Err(Error::not_directory(path)),
        } {
            let child = child.read().await;
            let file_name = Component::new(file_name);

            entries.push(Ok(DirEntry::new(
                self.child_path(path, &file_name)?,
                file_name,
                child.file_type(),
                child.metadata(),
            )));
        }

        Ok(stream::iter(entries))
    }

    async fn metadata(&self, path: &Path, options: &MetadataOptions) -> Result<Metadata, Error> {
        Ok(self
            .node(path, options.follows_symlinks())
            .await?
            .read()
            .await
            .metadata())
    }

    async fn access(&self, path: &Path, options: &AccessOptions) -> Result<(), Error> {
        options.evaluate(
            path,
            &self
                .node(path, options.follows_symlinks())
                .await?
                .read()
                .await
                .metadata(),
        )
    }

    async fn create_dir(&self, path: &Path, options: &CreateDirOptions) -> Result<(), Error> {
        if path.as_bytes() == b"/" {
            return Err(Error::already_exists(path));
        }

        if !options.is_recursive() {
            let (parent, file_name) = self.parent(path).await?;

            return match &mut *parent.write().await {
                Node::Directory(directory) => {
                    if directory
                        .entries()
                        .contains_key(file_name.as_bytes())
                    {
                        return Err(Error::already_exists(path));
                    }

                    directory.entries_mut().insert(
                        file_name.as_bytes().to_vec(),
                        Arc::new(RwLock::new(Node::directory(self.next_ino()))),
                    );
                    Ok(())
                }
                _ => Err(Error::not_directory(path)),
            };
        }

        let mut current = self.root.clone();

        for component in self.components(path) {
            current = {
                match &mut *current.write().await {
                    Node::Directory(directory) => match directory
                        .entries()
                        .get(component.as_bytes())
                        .cloned()
                    {
                        Some(node) => node,
                        None => {
                            let node = Arc::new(RwLock::new(Node::directory(self.next_ino())));

                            directory
                                .entries_mut()
                                .insert(component.as_bytes().to_vec(), node.clone());

                            node
                        }
                    },
                    _ => return Err(Error::not_directory(path)),
                }
            };
        }

        Ok(())
    }

    async fn remove_file(&self, path: &Path) -> Result<(), Error> {
        let (parent, file_name) = self.parent(path).await?;
        if matches!(
            &*self
                .child(&parent, path, &file_name)
                .await?
                .read()
                .await,
            Node::Directory(_)
        ) {
            return Err(Error::is_directory(path));
        }

        match &mut *parent.write().await {
            Node::Directory(directory) => {
                directory
                    .entries_mut()
                    .remove(file_name.as_bytes())
                    .ok_or_else(|| Error::not_found(path))?;

                Ok(())
            }
            _ => Err(Error::not_directory(path)),
        }
    }

    async fn remove_dir(&self, path: &Path, options: &RemoveDirOptions) -> Result<(), Error> {
        if path.as_bytes() == b"/" {
            return Err(Error::permission_denied(path));
        }

        let (parent, file_name) = self.parent(path).await?;
        match &*self
            .child(&parent, path, &file_name)
            .await?
            .read()
            .await
        {
            Node::Directory(child) if !options.is_recursive() && !child.entries().is_empty() => {
                return Err(Error::unsupported("directory is not empty"));
            }
            Node::Directory(_) => {}
            _ => return Err(Error::not_directory(path)),
        }

        match &mut *parent.write().await {
            Node::Directory(directory) => {
                directory
                    .entries_mut()
                    .remove(file_name.as_bytes())
                    .ok_or_else(|| Error::not_found(path))?;

                Ok(())
            }
            _ => Err(Error::not_directory(path)),
        }
    }

    async fn truncate(&self, path: &Path, len: u64) -> Result<(), Error> {
        match &mut *self
            .node(path, true)
            .await?
            .write()
            .await
        {
            Node::File(file) => {
                file.content()
                    .lock()
                    .map_err(|_| Error::unexpected("memory file lock is poisoned"))?
                    .resize(len as usize, 0);

                Ok(())
            }
            Node::Directory(_) => Err(Error::is_directory(path)),
            Node::Symlink(_) => Err(Error::not_found(path)),
        }
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<(), Error> {
        if from.as_bytes() == b"/" || to.as_bytes() == b"/" {
            return Err(Error::permission_denied(from));
        }

        let (from_parent, from_name) = self.parent(from).await?;
        let (to_parent, to_name) = self.parent(to).await?;

        if Arc::ptr_eq(&from_parent, &to_parent) {
            return match &mut *from_parent.write().await {
                Node::Directory(directory) => {
                    let entries = directory.entries_mut();

                    if entries.contains_key(to_name.as_bytes()) {
                        return Err(Error::already_exists(to));
                    }

                    if let Some(node) = entries.remove(from_name.as_bytes()) {
                        entries.insert(to_name.as_bytes().to_vec(), node);

                        Ok(())
                    } else {
                        Err(Error::not_found(from))
                    }
                }
                _ => Err(Error::not_directory(from)),
            };
        }

        match &*to_parent.read().await {
            Node::Directory(directory) => {
                if directory
                    .entries()
                    .contains_key(to_name.as_bytes())
                {
                    return Err(Error::already_exists(to));
                }
            }
            _ => return Err(Error::not_directory(to)),
        }

        let node = match &mut *from_parent.write().await {
            Node::Directory(directory) => directory
                .entries_mut()
                .remove(from_name.as_bytes())
                .ok_or_else(|| Error::not_found(from))?,
            _ => return Err(Error::not_directory(from)),
        };

        match &mut *to_parent.write().await {
            Node::Directory(directory) => {
                if directory
                    .entries()
                    .contains_key(to_name.as_bytes())
                {
                    return Err(Error::already_exists(to));
                }

                directory
                    .entries_mut()
                    .insert(to_name.as_bytes().to_vec(), node);

                Ok(())
            }
            _ => Err(Error::not_directory(to)),
        }
    }

    async fn symlink(&self, target: &Path, link: &Path) -> Result<(), Error> {
        let (parent, file_name) = self.parent(link).await?;

        match &mut *parent.write().await {
            Node::Directory(directory) => {
                if directory
                    .entries()
                    .contains_key(file_name.as_bytes())
                {
                    return Err(Error::already_exists(link));
                }

                directory.entries_mut().insert(
                    file_name.as_bytes().to_vec(),
                    Arc::new(RwLock::new(Node::symlink(
                        self.next_ino(),
                        PathBuf::parse(target.as_bytes())?,
                    ))),
                );

                Ok(())
            }
            _ => Err(Error::not_directory(link)),
        }
    }

    async fn read_link(&self, path: &Path) -> Result<PathBuf, Error> {
        match &*self
            .node(path, false)
            .await?
            .read()
            .await
        {
            Node::Symlink(symlink) => Ok(symlink.target()),
            _ => Err(Error::invalid_path("path is not a symlink")),
        }
    }

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> Result<(), Error> {
        self.node(path, false)
            .await?
            .write()
            .await
            .set_permissions(permissions);

        Ok(())
    }

    async fn set_owner(
        &self,
        path: &Path,
        owner: Owner,
        options: &SetOwnerOptions,
    ) -> Result<(), Error> {
        self.node(path, options.follows_symlinks())
            .await?
            .write()
            .await
            .set_owner(owner);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::sleep;

    use crate::{
        backend::Backend,
        errors::Error,
        fs::{
            AccessOptions, Dir, FileType, Fs, MetadataOptions, OpenOptions, Owner, Permissions,
            SetOwnerOptions,
        },
        memory::MemoryFs,
        path::PathBuf,
        readonly::ReadOnlyFs,
    };

    #[tokio::test]
    async fn creates_writes_and_reads_file() {
        let root = Fs::memory().root();
        let mut file = root
            .options()
            .write(true)
            .create(true)
            .open("notes.txt")
            .await
            .unwrap();

        file.write_all(b"hello").await.unwrap();
        file.flush().await.unwrap();

        let mut file = root
            .open_file("notes.txt")
            .await
            .unwrap();

        assert_eq!(file.read_to_string().await.unwrap(), "hello");
    }

    #[tokio::test]
    async fn lists_directory_entries() {
        let root = Fs::memory().root();

        root.create_dir("workspace")
            .await
            .unwrap();
        root.options()
            .write(true)
            .create(true)
            .open("notes.txt")
            .await
            .unwrap();

        let mut entries = root.entries().await.unwrap();
        let mut names = Vec::new();

        while let Some(entry) = entries.next().await.unwrap() {
            names.push(entry.file_name().to_string_lossy());
        }

        assert_eq!(names, vec!["notes.txt", "workspace"]);
    }

    #[tokio::test]
    async fn resolves_parent_paths_inside_authority() {
        let fs = Fs::memory();

        fs.root()
            .create_dir_all("/workspace/nested")
            .await
            .unwrap();

        assert_eq!(
            Dir::new(
                fs,
                PathBuf::parse("/workspace").unwrap(),
                PathBuf::parse("/workspace/nested").unwrap(),
            )
            .open_dir("..")
            .await
            .unwrap()
            .path()
            .to_string_lossy(),
            "/workspace"
        );
    }

    #[tokio::test]
    async fn rejects_authority_escape() {
        assert!(matches!(
            Dir::new(
                Fs::memory(),
                PathBuf::parse("/workspace").unwrap(),
                PathBuf::parse("/workspace").unwrap(),
            )
            .open_dir("..")
            .await,
            Err(Error::PathEscapesAuthority(path)) if path.to_string_lossy() == ".."
        ));
    }

    #[tokio::test]
    async fn creates_and_reads_symlink() {
        let root = Fs::memory().root();

        root.options()
            .write(true)
            .create(true)
            .open("target.txt")
            .await
            .unwrap();
        root.symlink("/target.txt", "link.txt")
            .await
            .unwrap();

        assert_eq!(
            root.read_link("link.txt")
                .await
                .unwrap()
                .to_string_lossy(),
            "/target.txt"
        );
    }

    #[tokio::test]
    async fn returns_metadata_for_node_types() {
        let root = Fs::memory().root();

        root.create_dir("workspace")
            .await
            .unwrap();
        root.options()
            .write(true)
            .create(true)
            .open("target.txt")
            .await
            .unwrap();
        root.symlink("/target.txt", "link.txt")
            .await
            .unwrap();

        assert_eq!(
            root.metadata("workspace")
                .await
                .unwrap()
                .file_type(),
            FileType::Directory
        );
        assert_eq!(
            root.metadata("target.txt")
                .await
                .unwrap()
                .file_type(),
            FileType::File
        );
        assert_eq!(
            root.entry("link.txt")
                .await
                .unwrap()
                .metadata()
                .unwrap()
                .file_type(),
            FileType::Symlink
        );
    }

    #[tokio::test]
    async fn removes_file_and_directory() {
        let root = Fs::memory().root();

        root.options()
            .write(true)
            .create(true)
            .open("notes.txt")
            .await
            .unwrap();
        root.create_dir("workspace")
            .await
            .unwrap();

        root.remove_file("notes.txt")
            .await
            .unwrap();
        root.remove_dir("workspace")
            .await
            .unwrap();

        assert!(matches!(
            root.metadata("notes.txt").await,
            Err(Error::NotFound(path)) if path.to_string_lossy() == "/notes.txt"
        ));
        assert!(matches!(
            root.metadata("workspace").await,
            Err(Error::NotFound(path)) if path.to_string_lossy() == "/workspace"
        ));
    }

    #[tokio::test]
    async fn renames_node_in_same_backend() {
        let root = Fs::memory().root();

        root.options()
            .write(true)
            .create(true)
            .open("before.txt")
            .await
            .unwrap();

        root.rename("before.txt", "after.txt")
            .await
            .unwrap();

        assert_eq!(
            root.metadata("after.txt")
                .await
                .unwrap()
                .file_type(),
            FileType::File
        );
        assert!(matches!(
            root.metadata("before.txt").await,
            Err(Error::NotFound(path)) if path.to_string_lossy() == "/before.txt"
        ));
    }

    #[tokio::test]
    async fn memory_truncate_shortens_a_file_by_path() {
        let root = Fs::memory().root();

        root.options()
            .write(true)
            .create(true)
            .open("/notes.txt")
            .await
            .unwrap()
            .write_all(b"hello world")
            .await
            .unwrap();

        root.truncate("/notes.txt", 5).await.unwrap();

        assert_eq!(
            root.open_file("/notes.txt")
                .await
                .unwrap()
                .read_to_string()
                .await
                .unwrap(),
            "hello"
        );
    }

    #[tokio::test]
    async fn memory_truncate_rejects_a_directory() {
        let root = Fs::memory().root();

        root.create_dir("/workspace")
            .await
            .unwrap();

        assert!(matches!(
            root.truncate("/workspace", 0).await,
            Err(Error::IsDirectory(path)) if path.to_string_lossy() == "/workspace"
        ));
    }

    #[tokio::test]
    async fn memory_set_owner_changes_both_members() {
        let root = Fs::memory().root();

        root.options()
            .write(true)
            .create(true)
            .open("/notes.txt")
            .await
            .unwrap();
        root.set_owner(
            "/notes.txt",
            Owner::new()
                .user(1000)
                .group(1000),
        )
        .await
        .unwrap();

        assert_eq!(root.metadata("/notes.txt").await.unwrap().uid(), 1000);
        assert_eq!(root.metadata("/notes.txt").await.unwrap().gid(), 1000);
    }

    #[tokio::test]
    async fn memory_set_owner_leaves_unset_members_alone() {
        let root = Fs::memory().root();

        root.options()
            .write(true)
            .create(true)
            .open("/notes.txt")
            .await
            .unwrap();
        root.set_owner(
            "/notes.txt",
            Owner::new()
                .user(1000)
                .group(1000),
        )
        .await
        .unwrap();
        root.set_owner("/notes.txt", Owner::new().group(2000))
            .await
            .unwrap();

        assert_eq!(root.metadata("/notes.txt").await.unwrap().uid(), 1000);
        assert_eq!(root.metadata("/notes.txt").await.unwrap().gid(), 2000);
    }

    #[tokio::test]
    async fn memory_set_owner_on_a_symlink_follows_by_default() {
        let root = Fs::memory().root();

        root.options()
            .write(true)
            .create(true)
            .open("/target.txt")
            .await
            .unwrap();
        root.symlink("/target.txt", "/link.txt")
            .await
            .unwrap();
        root.set_owner("/link.txt", Owner::new().user(1000))
            .await
            .unwrap();

        assert_eq!(root.metadata("/target.txt").await.unwrap().uid(), 1000);
        assert_eq!(root.metadata("/link.txt").await.unwrap().uid(), 0);

        root.set_owner_with_options(
            "/link.txt",
            Owner::new().user(2000),
            &SetOwnerOptions::new().follow_symlinks(false),
        )
        .await
        .unwrap();

        assert_eq!(root.metadata("/link.txt").await.unwrap().uid(), 2000);
    }

    #[tokio::test]
    async fn memory_reports_ownership_capability() {
        assert!(Fs::memory().backend.capabilities().supports_owner());
    }

    #[tokio::test]
    async fn memory_nodes_carry_posix_modes() {
        let root = Fs::memory().root();

        root.create_dir("/workspace")
            .await
            .unwrap();
        root.options()
            .write(true)
            .create(true)
            .open("/notes.txt")
            .await
            .unwrap();
        root.symlink("/notes.txt", "/notes-link.txt")
            .await
            .unwrap();

        assert_eq!(
            root.metadata("/workspace")
                .await
                .unwrap()
                .permissions()
                .mode(),
            Permissions::DIRECTORY
        );
        assert_eq!(
            root.metadata("/notes.txt")
                .await
                .unwrap()
                .permissions()
                .mode(),
            Permissions::FILE
        );
        assert_eq!(
            root.entry("/notes-link.txt")
                .await
                .unwrap()
                .metadata()
                .unwrap()
                .permissions()
                .mode(),
            Permissions::SYMLINK
        );
    }

    #[tokio::test]
    async fn memory_entries_receive_distinct_inode_numbers() {
        let root = Fs::memory().root();

        root.options()
            .write(true)
            .create(true)
            .open("/first.txt")
            .await
            .unwrap();
        root.options()
            .write(true)
            .create(true)
            .open("/second.txt")
            .await
            .unwrap();

        let first = root.metadata("/first.txt").await.unwrap().ino();
        let second = root.metadata("/second.txt").await.unwrap().ino();

        assert_ne!(first, 0);
        assert_ne!(second, 0);
        assert_ne!(first, second);
    }

    #[tokio::test]
    async fn memory_root_has_the_first_inode_number() {
        assert_eq!(Fs::memory().root().metadata("/").await.unwrap().ino(), 1);
    }

    #[tokio::test]
    async fn memory_permission_change_updates_the_status_timestamp() {
        let fs = MemoryFs::new();
        let path = PathBuf::parse("/notes.txt").unwrap();

        Backend::open(
            &fs,
            path.as_path(),
            &OpenOptions::new()
                .write(true)
                .create(true),
        )
        .await
        .unwrap();

        let before = Backend::metadata(&fs, path.as_path(), &MetadataOptions::new())
            .await
            .unwrap()
            .changed()
            .unwrap();

        sleep(Duration::from_millis(1)).await;

        Backend::set_permissions(&fs, path.as_path(), Permissions::new(0o600))
            .await
            .unwrap();

        assert!(
            Backend::metadata(&fs, path.as_path(), &MetadataOptions::new())
                .await
                .unwrap()
                .changed()
                .unwrap()
                > before
        );
    }

    #[tokio::test]
    async fn set_permissions_changes_the_reported_mode() {
        let root = Fs::memory().root();

        root.options()
            .write(true)
            .create(true)
            .open("/notes.txt")
            .await
            .unwrap();
        root.set_permissions("/notes.txt", Permissions::new(0o600))
            .await
            .unwrap();

        assert_eq!(
            root.metadata("/notes.txt")
                .await
                .unwrap()
                .permissions()
                .mode(),
            0o600
        );
    }

    #[tokio::test]
    async fn set_permissions_rejects_paths_outside_the_authority() {
        assert!(matches!(
            Dir::new(
                Fs::memory(),
                PathBuf::parse("/workspace").unwrap(),
                PathBuf::parse("/workspace").unwrap(),
            )
            .set_permissions("..", Permissions::new(0o600))
            .await,
            Err(Error::PathEscapesAuthority(path)) if path.to_string_lossy() == ".."
        ));
    }

    #[tokio::test]
    async fn set_permissions_is_refused_by_a_readonly_backend() {
        assert!(matches!(
            Fs::new(ReadOnlyFs::new(MemoryFs::new()))
                .root()
                .set_permissions("/notes.txt", Permissions::new(0o600))
                .await,
            Err(Error::PermissionDenied(path)) if path.to_string_lossy() == "/notes.txt"
        ));
    }

    #[tokio::test]
    async fn memory_entries_report_symlinks_as_symlinks() {
        let root = Fs::memory().root();

        root.create_dir("/target")
            .await
            .unwrap();
        root.symlink("/target", "/link")
            .await
            .unwrap();

        let mut entries = root.entries().await.unwrap();
        let mut file_types = Vec::new();

        while let Some(entry) = entries.next().await.unwrap() {
            file_types.push((entry.file_name().to_string_lossy(), entry.file_type()));
        }

        assert!(file_types.contains(&("link".to_string(), FileType::Symlink)));
    }

    #[tokio::test]
    async fn access_permits_existing_path_with_no_flags() {
        let root = Fs::memory().root();

        root.options()
            .write(true)
            .create(true)
            .open("/notes.txt")
            .await
            .unwrap();

        root.access("/notes.txt", &AccessOptions::new())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn access_reports_missing_path_as_not_found() {
        assert!(matches!(
            Fs::memory()
                .root()
                .access("/missing.txt", &AccessOptions::new())
                .await,
            Err(Error::NotFound(path)) if path.to_string_lossy() == "/missing.txt"
        ));
    }

    #[tokio::test]
    async fn access_denies_write_on_a_readonly_mode() {
        let root = Fs::memory().root();

        root.options()
            .write(true)
            .create(true)
            .open("/notes.txt")
            .await
            .unwrap();
        root.set_permissions("/notes.txt", Permissions::new(0o444))
            .await
            .unwrap();

        assert!(matches!(
            root.access("/notes.txt", &AccessOptions::new().write(true))
                .await,
            Err(Error::PermissionDenied(path)) if path.to_string_lossy() == "/notes.txt"
        ));
    }

    #[tokio::test]
    async fn access_permits_execute_on_executable_mode() {
        let root = Fs::memory().root();

        root.options()
            .write(true)
            .create(true)
            .open("/script.sh")
            .await
            .unwrap();
        root.set_permissions("/script.sh", Permissions::new(0o755))
            .await
            .unwrap();

        root.access("/script.sh", &AccessOptions::new().execute(true))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn walk_yields_every_entry_beneath_a_directory() {
        let root = Fs::memory().root();

        root.create_dir_all("/a/b")
            .await
            .unwrap();
        root.options()
            .write(true)
            .create(true)
            .open("/a/b/c.txt")
            .await
            .unwrap();
        root.options()
            .write(true)
            .create(true)
            .open("/a/d.txt")
            .await
            .unwrap();

        let mut walk = root.walk().await.unwrap();
        let mut paths = Vec::new();

        while let Some(entry) = walk.next().await.unwrap() {
            paths.push(entry.path().to_string_lossy());
        }

        paths.sort();

        assert_eq!(paths, vec!["/a", "/a/b", "/a/b/c.txt", "/a/d.txt"]);
    }

    #[tokio::test]
    async fn walk_yields_directories_as_well_as_their_contents() {
        let root = Fs::memory().root();

        root.create_dir_all("/a/b")
            .await
            .unwrap();
        root.options()
            .write(true)
            .create(true)
            .open("/a/b/c.txt")
            .await
            .unwrap();

        let mut walk = root.walk().await.unwrap();
        let mut paths = Vec::new();

        while let Some(entry) = walk.next().await.unwrap() {
            paths.push(entry.path().to_string_lossy());
        }

        assert!(paths.contains(&"/a/b".to_string()));
    }

    #[tokio::test]
    async fn walk_does_not_descend_into_symlinked_directories() {
        let root = Fs::memory().root();

        root.create_dir("/real")
            .await
            .unwrap();
        root.options()
            .write(true)
            .create(true)
            .open("/real/file.txt")
            .await
            .unwrap();
        root.symlink("/real", "/link")
            .await
            .unwrap();

        let mut walk = root.walk().await.unwrap();
        let mut entries = Vec::new();

        while let Some(entry) = walk.next().await.unwrap() {
            entries.push((entry.path().to_string_lossy(), entry.file_type()));
        }

        assert!(entries.contains(&("/link".to_string(), FileType::Symlink)));
        assert!(!entries
            .iter()
            .any(|(path, _)| path == "/link/file.txt"));
    }

    #[tokio::test]
    async fn walk_over_an_empty_directory_yields_nothing() {
        let mut walk = Fs::memory()
            .root()
            .walk()
            .await
            .unwrap();

        assert!(walk.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn create_dir_temp_appends_six_characters_to_the_prefix() {
        let root = Fs::memory().root();

        root.create_dir("/tmp")
            .await
            .unwrap();

        let dir = root.create_dir_temp("/tmp/run-").await.unwrap();
        let path = dir.path().to_string_lossy();

        assert!(path.starts_with("/tmp/run-"));
        assert_eq!(path.len(), "/tmp/run-".len() + 6);
    }

    #[tokio::test]
    async fn create_dir_temp_returns_distinct_paths() {
        let root = Fs::memory().root();

        root.create_dir("/tmp")
            .await
            .unwrap();

        let first = root.create_dir_temp("/tmp/run-").await.unwrap();
        let second = root.create_dir_temp("/tmp/run-").await.unwrap();

        assert_ne!(first.path(), second.path());
        root.metadata(first.path()).await.unwrap();
        root.metadata(second.path()).await.unwrap();
    }

    #[tokio::test]
    async fn create_dir_temp_restricts_the_mode() {
        let root = Fs::memory().root();

        root.create_dir("/tmp")
            .await
            .unwrap();

        assert_eq!(
            root.create_dir_temp("/tmp/run-")
                .await
                .unwrap()
                .metadata(".")
                .await
                .unwrap()
                .permissions()
                .mode(),
            0o700
        );
    }

    #[tokio::test]
    async fn create_dir_temp_uses_only_alphabet_characters() {
        let root = Fs::memory().root();

        root.create_dir("/tmp")
            .await
            .unwrap();

        assert!(
            root.create_dir_temp("/tmp/run-")
                .await
                .unwrap()
                .path()
                .to_string_lossy()
                .strip_prefix("/tmp/run-")
                .unwrap()
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric())
        );
    }

    #[tokio::test]
    async fn create_dir_temp_rejects_paths_outside_the_authority() {
        assert!(matches!(
            Dir::new(
                Fs::memory(),
                PathBuf::parse("/workspace").unwrap(),
                PathBuf::parse("/workspace").unwrap(),
            )
            .create_dir_temp("../tmp-")
            .await,
            Err(Error::PathEscapesAuthority(path)) if path.to_string_lossy() == "../tmp-"
        ));
    }
}
