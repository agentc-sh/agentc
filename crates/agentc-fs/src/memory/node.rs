// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::SystemTime,
};

use tokio::sync::RwLock;

use crate::{
    fs::{FileType, Metadata, Permissions},
    path::PathBuf,
};

pub(crate) type NodeRef = Arc<RwLock<Node>>;

pub(crate) enum Node {
    File(MemoryFileNode),
    Directory(MemoryDirectoryNode),
    Symlink(MemorySymlinkNode),
}

impl Node {
    pub(crate) fn directory() -> Self {
        Node::Directory(MemoryDirectoryNode::new())
    }

    pub(crate) fn file() -> Self {
        Node::File(MemoryFileNode::new())
    }

    pub(crate) fn symlink(target: PathBuf) -> Self {
        Node::Symlink(MemorySymlinkNode::new(target))
    }

    pub(crate) fn file_type(&self) -> FileType {
        match self {
            Node::File(_) => FileType::File,
            Node::Directory(_) => FileType::Directory,
            Node::Symlink(_) => FileType::Symlink,
        }
    }

    pub(crate) fn metadata(&self) -> Metadata {
        match self {
            Node::File(node) => node.metadata(FileType::File),
            Node::Directory(node) => node.metadata(FileType::Directory),
            Node::Symlink(node) => node.metadata(FileType::Symlink),
        }
    }

    pub(crate) fn set_permissions(&mut self, permissions: Permissions) {
        match self {
            Node::File(node) => node.permissions = permissions,
            Node::Directory(node) => node.permissions = permissions,
            Node::Symlink(node) => node.permissions = permissions,
        }
    }
}

pub(crate) struct MemoryFileNode {
    content: Arc<Mutex<Vec<u8>>>,
    permissions: Permissions,
    accessed: Option<SystemTime>,
    modified: Option<SystemTime>,
    created: Option<SystemTime>,
}

impl MemoryFileNode {
    fn new() -> Self {
        let now = SystemTime::now();

        MemoryFileNode {
            content: Arc::new(Mutex::new(Vec::new())),
            permissions: Permissions::new(),
            accessed: Some(now),
            modified: Some(now),
            created: Some(now),
        }
    }

    pub(crate) fn content(&self) -> Arc<Mutex<Vec<u8>>> {
        self.content.clone()
    }

    pub(crate) fn metadata(&self, file_type: FileType) -> Metadata {
        Metadata::new(
            file_type,
            self.content
                .lock()
                .map(|content| content.len() as u64)
                .unwrap_or_default(),
            self.permissions,
        )
        .with_accessed(self.accessed)
        .with_modified(self.modified)
        .with_created(self.created)
    }
}

pub(crate) struct MemoryDirectoryNode {
    entries: BTreeMap<Vec<u8>, NodeRef>,
    permissions: Permissions,
    accessed: Option<SystemTime>,
    modified: Option<SystemTime>,
    created: Option<SystemTime>,
}

impl MemoryDirectoryNode {
    fn new() -> Self {
        let now = SystemTime::now();

        MemoryDirectoryNode {
            entries: BTreeMap::new(),
            permissions: Permissions::new(),
            accessed: Some(now),
            modified: Some(now),
            created: Some(now),
        }
    }

    pub(crate) fn entries(&self) -> &BTreeMap<Vec<u8>, NodeRef> {
        &self.entries
    }

    pub(crate) fn entries_mut(&mut self) -> &mut BTreeMap<Vec<u8>, NodeRef> {
        &mut self.entries
    }

    pub(crate) fn metadata(&self, file_type: FileType) -> Metadata {
        Metadata::new(file_type, self.entries.len() as u64, self.permissions)
            .with_accessed(self.accessed)
            .with_modified(self.modified)
            .with_created(self.created)
    }
}

pub(crate) struct MemorySymlinkNode {
    target: PathBuf,
    permissions: Permissions,
    accessed: Option<SystemTime>,
    modified: Option<SystemTime>,
    created: Option<SystemTime>,
}

impl MemorySymlinkNode {
    fn new(target: PathBuf) -> Self {
        let now = SystemTime::now();

        MemorySymlinkNode {
            target,
            permissions: Permissions::new(),
            accessed: Some(now),
            modified: Some(now),
            created: Some(now),
        }
    }

    pub(crate) fn target(&self) -> PathBuf {
        self.target.clone()
    }

    pub(crate) fn metadata(&self, file_type: FileType) -> Metadata {
        Metadata::new(file_type, self.target.as_bytes().len() as u64, self.permissions)
            .with_accessed(self.accessed)
            .with_modified(self.modified)
            .with_created(self.created)
    }
}
