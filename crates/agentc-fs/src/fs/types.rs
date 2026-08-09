// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::time::SystemTime;

use crate::{
    errors::Error,
    fs::{dir::Dir, file::File},
    path::IntoPathBuf,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Metadata {
    file_type: FileType,
    len: u64,
    permissions: Permissions,
    accessed: Option<SystemTime>,
    modified: Option<SystemTime>,
    created: Option<SystemTime>,
}

impl Metadata {
    pub fn new(file_type: FileType, len: u64, permissions: Permissions) -> Self {
        Metadata {
            file_type,
            len,
            permissions,
            accessed: None,
            modified: None,
            created: None,
        }
    }

    pub fn file_type(&self) -> FileType {
        self.file_type
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn permissions(&self) -> &Permissions {
        &self.permissions
    }

    pub fn accessed(&self) -> Option<SystemTime> {
        self.accessed
    }

    pub fn modified(&self) -> Option<SystemTime> {
        self.modified
    }

    pub fn created(&self) -> Option<SystemTime> {
        self.created
    }

    pub fn with_accessed(mut self, accessed: impl Into<Option<SystemTime>>) -> Self {
        self.accessed = accessed.into();
        self
    }

    pub fn with_modified(mut self, modified: impl Into<Option<SystemTime>>) -> Self {
        self.modified = modified.into();
        self
    }

    pub fn with_created(mut self, created: impl Into<Option<SystemTime>>) -> Self {
        self.created = created.into();
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileType {
    File,
    Directory,
    Symlink,
    Fifo,
    Socket,
    BlockDevice,
    CharacterDevice,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Permissions {
    readonly: bool,
    mode: Option<u32>,
}

impl Permissions {
    pub fn new() -> Self {
        Permissions { readonly: false, mode: None }
    }

    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    pub fn mode(mut self, mode: impl Into<Option<u32>>) -> Self {
        self.mode = mode.into();
        self
    }

    pub fn is_readonly(&self) -> bool {
        self.readonly
    }

    pub fn posix_mode(&self) -> Option<u32> {
        self.mode
    }
}

impl Default for Permissions {
    fn default() -> Self {
        Permissions::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PermissionCapability {
    None,
    Readonly,
    PosixMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Capabilities {
    symlink: bool,
    hard_link: bool,
    atomic_rename: bool,
    permissions: PermissionCapability,
    timestamps: bool,
}

impl Capabilities {
    pub fn new() -> Self {
        Capabilities {
            symlink: false,
            hard_link: false,
            atomic_rename: false,
            permissions: PermissionCapability::None,
            timestamps: false,
        }
    }

    pub fn symlink(mut self, symlink: bool) -> Self {
        self.symlink = symlink;
        self
    }

    pub fn hard_link(mut self, hard_link: bool) -> Self {
        self.hard_link = hard_link;
        self
    }

    pub fn atomic_rename(mut self, atomic_rename: bool) -> Self {
        self.atomic_rename = atomic_rename;
        self
    }

    pub fn permissions(mut self, permissions: PermissionCapability) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn timestamps(mut self, timestamps: bool) -> Self {
        self.timestamps = timestamps;
        self
    }

    pub fn supports_symlink(&self) -> bool {
        self.symlink
    }

    pub fn supports_hard_link(&self) -> bool {
        self.hard_link
    }

    pub fn supports_atomic_rename(&self) -> bool {
        self.atomic_rename
    }

    pub fn permission_capability(&self) -> PermissionCapability {
        self.permissions
    }

    pub fn supports_timestamps(&self) -> bool {
        self.timestamps
    }
}

impl Default for Capabilities {
    fn default() -> Self {
        Capabilities::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,
    follow_symlinks: bool,
}

impl OpenOptions {
    pub fn new() -> Self {
        OpenOptions {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
            follow_symlinks: true,
        }
    }

    pub fn read(mut self, read: bool) -> Self {
        self.read = read;
        self
    }

    pub fn write(mut self, write: bool) -> Self {
        self.write = write;
        self
    }

    pub fn append(mut self, append: bool) -> Self {
        self.append = append;
        self
    }

    pub fn truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }

    pub fn create(mut self, create: bool) -> Self {
        self.create = create;
        self
    }

    pub fn create_new(mut self, create_new: bool) -> Self {
        self.create_new = create_new;
        self
    }

    pub fn follow_symlinks(mut self, follow_symlinks: bool) -> Self {
        self.follow_symlinks = follow_symlinks;
        self
    }

    pub fn is_read(&self) -> bool {
        self.read
    }

    pub fn is_write(&self) -> bool {
        self.write
    }

    pub fn is_append(&self) -> bool {
        self.append
    }

    pub fn is_truncate(&self) -> bool {
        self.truncate
    }

    pub fn is_create(&self) -> bool {
        self.create
    }

    pub fn is_create_new(&self) -> bool {
        self.create_new
    }

    pub fn follows_symlinks(&self) -> bool {
        self.follow_symlinks
    }
}

impl Default for OpenOptions {
    fn default() -> Self {
        OpenOptions::new()
    }
}

pub struct OpenOptionsBuilder<'a> {
    dir: &'a Dir,
    options: OpenOptions,
}

impl<'a> OpenOptionsBuilder<'a> {
    pub fn new(dir: &'a Dir) -> Self {
        OpenOptionsBuilder { dir, options: OpenOptions::new() }
    }

    pub fn read(mut self, read: bool) -> Self {
        self.options = self.options.read(read);
        self
    }

    pub fn write(mut self, write: bool) -> Self {
        self.options = self.options.write(write);
        self
    }

    pub fn append(mut self, append: bool) -> Self {
        self.options = self.options.append(append);
        self
    }

    pub fn truncate(mut self, truncate: bool) -> Self {
        self.options = self.options.truncate(truncate);
        self
    }

    pub fn create(mut self, create: bool) -> Self {
        self.options = self.options.create(create);
        self
    }

    pub fn create_new(mut self, create_new: bool) -> Self {
        self.options = self.options.create_new(create_new);
        self
    }

    pub fn follow_symlinks(mut self, follow_symlinks: bool) -> Self {
        self.options = self
            .options
            .follow_symlinks(follow_symlinks);
        self
    }

    pub async fn open(self, path: impl IntoPathBuf) -> Result<File, Error> {
        self.dir
            .open_with_options(path, &self.options)
            .await
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateDirOptions {
    recursive: bool,
}

impl CreateDirOptions {
    pub fn new() -> Self {
        CreateDirOptions { recursive: false }
    }

    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    pub fn is_recursive(&self) -> bool {
        self.recursive
    }
}

impl Default for CreateDirOptions {
    fn default() -> Self {
        CreateDirOptions::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoveDirOptions {
    recursive: bool,
}

impl RemoveDirOptions {
    pub fn new() -> Self {
        RemoveDirOptions { recursive: false }
    }

    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    pub fn is_recursive(&self) -> bool {
        self.recursive
    }
}

impl Default for RemoveDirOptions {
    fn default() -> Self {
        RemoveDirOptions::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetadataOptions {
    follow_symlinks: bool,
}

impl MetadataOptions {
    pub fn new() -> Self {
        MetadataOptions { follow_symlinks: false }
    }

    pub fn follow_symlinks(mut self, follow_symlinks: bool) -> Self {
        self.follow_symlinks = follow_symlinks;
        self
    }

    pub fn follows_symlinks(&self) -> bool {
        self.follow_symlinks
    }
}

impl Default for MetadataOptions {
    fn default() -> Self {
        MetadataOptions::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymlinkOptions {
    overwrite: bool,
}

impl SymlinkOptions {
    pub fn new() -> Self {
        SymlinkOptions { overwrite: false }
    }

    pub fn overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    pub fn is_overwrite(&self) -> bool {
        self.overwrite
    }
}

impl Default for SymlinkOptions {
    fn default() -> Self {
        SymlinkOptions::new()
    }
}
