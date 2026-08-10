// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::{
    fs::{CreateDirOptions, MetadataOptions, OpenOptions, Owner, Permissions, RemoveDirOptions},
    path::Path,
};

pub struct OpenContext<'a> {
    path: &'a Path,
    options: &'a OpenOptions,
}

impl<'a> OpenContext<'a> {
    pub fn new(path: &'a Path, options: &'a OpenOptions) -> Self {
        OpenContext { path, options }
    }

    pub fn path(&self) -> &Path {
        self.path
    }

    pub fn options(&self) -> &OpenOptions {
        self.options
    }
}

pub struct EntriesContext<'a> {
    path: &'a Path,
}

impl<'a> EntriesContext<'a> {
    pub fn new(path: &'a Path) -> Self {
        EntriesContext { path }
    }

    pub fn path(&self) -> &Path {
        self.path
    }
}

pub struct MetadataContext<'a> {
    path: &'a Path,
    options: &'a MetadataOptions,
}

impl<'a> MetadataContext<'a> {
    pub fn new(path: &'a Path, options: &'a MetadataOptions) -> Self {
        MetadataContext { path, options }
    }

    pub fn path(&self) -> &Path {
        self.path
    }

    pub fn options(&self) -> &MetadataOptions {
        self.options
    }
}

pub struct WriteContext<'a> {
    path: &'a Path,
    open_options: Option<&'a OpenOptions>,
    create_dir_options: Option<&'a CreateDirOptions>,
    permissions: Option<&'a Permissions>,
    owner: Option<&'a Owner>,
}

impl<'a> WriteContext<'a> {
    pub fn new(
        path: &'a Path,
        open_options: impl Into<Option<&'a OpenOptions>>,
        create_dir_options: impl Into<Option<&'a CreateDirOptions>>,
        permissions: impl Into<Option<&'a Permissions>>,
        owner: impl Into<Option<&'a Owner>>,
    ) -> Self {
        WriteContext {
            path,
            open_options: open_options.into(),
            create_dir_options: create_dir_options.into(),
            permissions: permissions.into(),
            owner: owner.into(),
        }
    }

    pub fn path(&self) -> &Path {
        self.path
    }

    pub fn open_options(&self) -> Option<&OpenOptions> {
        self.open_options
    }

    pub fn create_dir_options(&self) -> Option<&CreateDirOptions> {
        self.create_dir_options
    }

    pub fn permissions(&self) -> Option<&Permissions> {
        self.permissions
    }

    pub fn owner(&self) -> Option<&Owner> {
        self.owner
    }
}

pub struct RemoveContext<'a> {
    path: &'a Path,
    options: Option<&'a RemoveDirOptions>,
}

impl<'a> RemoveContext<'a> {
    pub fn new(path: &'a Path, options: impl Into<Option<&'a RemoveDirOptions>>) -> Self {
        RemoveContext { path, options: options.into() }
    }

    pub fn path(&self) -> &Path {
        self.path
    }

    pub fn options(&self) -> Option<&RemoveDirOptions> {
        self.options
    }
}

pub struct RenameContext<'a> {
    from: &'a Path,
    to: &'a Path,
}

impl<'a> RenameContext<'a> {
    pub fn new(from: &'a Path, to: &'a Path) -> Self {
        RenameContext { from, to }
    }

    pub fn from(&self) -> &Path {
        self.from
    }

    pub fn to(&self) -> &Path {
        self.to
    }
}

pub struct SymlinkContext<'a> {
    target: &'a Path,
    link: &'a Path,
}

impl<'a> SymlinkContext<'a> {
    pub fn new(target: &'a Path, link: &'a Path) -> Self {
        SymlinkContext { target, link }
    }

    pub fn target(&self) -> &Path {
        self.target
    }

    pub fn link(&self) -> &Path {
        self.link
    }
}
