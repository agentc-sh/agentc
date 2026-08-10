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
    changed: Option<SystemTime>,
    dev: u64,
    ino: u64,
    nlink: u64,
    uid: u32,
    gid: u32,
    rdev: u64,
    blksize: u64,
    blocks: u64,
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
            changed: None,
            dev: 0,
            ino: 0,
            nlink: 1,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 4096,
            blocks: len.div_ceil(512),
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

    pub fn changed(&self) -> Option<SystemTime> {
        self.changed
    }

    pub fn dev(&self) -> u64 {
        self.dev
    }

    pub fn ino(&self) -> u64 {
        self.ino
    }

    pub fn nlink(&self) -> u64 {
        self.nlink
    }

    pub fn uid(&self) -> u32 {
        self.uid
    }

    pub fn gid(&self) -> u32 {
        self.gid
    }

    pub fn rdev(&self) -> u64 {
        self.rdev
    }

    pub fn blksize(&self) -> u64 {
        self.blksize
    }

    pub fn blocks(&self) -> u64 {
        self.blocks
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

    pub fn with_changed(mut self, changed: impl Into<Option<SystemTime>>) -> Self {
        self.changed = changed.into();
        self
    }

    pub fn with_permissions(mut self, permissions: Permissions) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn with_dev(mut self, dev: u64) -> Self {
        self.dev = dev;
        self
    }

    pub fn with_ino(mut self, ino: u64) -> Self {
        self.ino = ino;
        self
    }

    pub fn with_nlink(mut self, nlink: u64) -> Self {
        self.nlink = nlink;
        self
    }

    pub fn with_uid(mut self, uid: u32) -> Self {
        self.uid = uid;
        self
    }

    pub fn with_gid(mut self, gid: u32) -> Self {
        self.gid = gid;
        self
    }

    pub fn with_rdev(mut self, rdev: u64) -> Self {
        self.rdev = rdev;
        self
    }

    pub fn with_blksize(mut self, blksize: u64) -> Self {
        self.blksize = blksize;
        self
    }

    pub fn with_blocks(mut self, blocks: u64) -> Self {
        self.blocks = blocks;
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
    mode: u32,
}

impl Permissions {
    pub const FILE: u32 = 0o644;
    pub const DIRECTORY: u32 = 0o755;
    pub const SYMLINK: u32 = 0o777;

    const MASK: u32 = 0o7777;
    const WRITE: u32 = 0o222;

    pub fn new(mode: u32) -> Self {
        Permissions {
            mode: mode & Self::MASK,
        }
    }

    pub fn mode(&self) -> u32 {
        self.mode
    }

    pub fn is_readonly(&self) -> bool {
        self.mode & Self::WRITE == 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Capabilities {
    symlink: bool,
    hard_link: bool,
    atomic_rename: bool,
    permissions: bool,
    owner: bool,
    timestamps: bool,
}

impl Capabilities {
    pub fn new() -> Self {
        Capabilities {
            symlink: false,
            hard_link: false,
            atomic_rename: false,
            permissions: false,
            owner: false,
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

    pub fn permissions(mut self, permissions: bool) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn owner(mut self, owner: bool) -> Self {
        self.owner = owner;
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

    pub fn supports_permissions(&self) -> bool {
        self.permissions
    }

    pub fn supports_owner(&self) -> bool {
        self.owner
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Owner {
    user: Option<u32>,
    group: Option<u32>,
}

impl Owner {
    pub fn new() -> Self {
        Owner::default()
    }

    pub fn user(mut self, user: impl Into<Option<u32>>) -> Self {
        self.user = user.into();
        self
    }

    pub fn group(mut self, group: impl Into<Option<u32>>) -> Self {
        self.group = group.into();
        self
    }

    pub fn user_id(&self) -> Option<u32> {
        self.user
    }

    pub fn group_id(&self) -> Option<u32> {
        self.group
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SetOwnerOptions {
    follow_symlinks: bool,
}

impl SetOwnerOptions {
    pub fn new() -> Self {
        SetOwnerOptions { follow_symlinks: true }
    }

    pub fn follow_symlinks(mut self, follow_symlinks: bool) -> Self {
        self.follow_symlinks = follow_symlinks;
        self
    }

    pub fn follows_symlinks(&self) -> bool {
        self.follow_symlinks
    }
}

impl Default for SetOwnerOptions {
    fn default() -> Self {
        SetOwnerOptions::new()
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

#[cfg(test)]
mod tests {
    use crate::fs::{FileType, Metadata, Owner, Permissions, SetOwnerOptions};

    #[test]
    fn permissions_mask_discards_file_type_bits() {
        assert_eq!(Permissions::new(0o100644).mode(), 0o644);
    }

    #[test]
    fn permissions_without_write_bits_are_readonly() {
        assert!(Permissions::new(0o444).is_readonly());
        assert!(!Permissions::new(0o644).is_readonly());
    }

    #[test]
    fn permissions_keep_set_user_bits() {
        assert_eq!(Permissions::new(0o4755).mode(), 0o4755);
    }

    #[test]
    fn metadata_defaults_are_posix_correct() {
        let metadata = Metadata::new(
            FileType::File,
            1000,
            Permissions::new(Permissions::FILE),
        );

        assert_eq!(metadata.nlink(), 1);
        assert_eq!(metadata.blksize(), 4096);
        assert_eq!(metadata.blocks(), 2);
        assert_eq!(metadata.dev(), 0);
        assert_eq!(metadata.ino(), 0);
        assert_eq!(metadata.rdev(), 0);
    }

    #[test]
    fn metadata_blocks_round_up_to_whole_blocks() {
        assert_eq!(
            Metadata::new(FileType::File, 0, Permissions::new(Permissions::FILE)).blocks(),
            0
        );
        assert_eq!(
            Metadata::new(FileType::File, 1, Permissions::new(Permissions::FILE)).blocks(),
            1
        );
        assert_eq!(
            Metadata::new(FileType::File, 512, Permissions::new(Permissions::FILE)).blocks(),
            1
        );
        assert_eq!(
            Metadata::new(FileType::File, 513, Permissions::new(Permissions::FILE)).blocks(),
            2
        );
    }

    #[test]
    fn owner_default_changes_nothing() {
        let owner = Owner::default();

        assert_eq!(owner.user_id(), None);
        assert_eq!(owner.group_id(), None);
    }

    #[test]
    fn set_owner_options_follow_symlinks_by_default() {
        assert!(SetOwnerOptions::new().follows_symlinks());
    }
}
