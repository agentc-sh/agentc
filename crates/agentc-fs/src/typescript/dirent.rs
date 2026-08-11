// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::guestjs::host::{ClassSpec, HostClass};

use crate::fs::{DirEntry, FileType};

pub struct Dirent {
    name: String,
    parent_path: String,
    file_type: FileType,
}

impl Dirent {
    pub fn new(entry: &DirEntry) -> Self {
        Self {
            name: entry
                .file_name()
                .to_string_lossy()
                .to_owned(),
            parent_path: entry
                .path()
                .parent()
                .map(|path| path.to_string_lossy().to_owned())
                .unwrap_or_else(|| "/".to_owned()),
            file_type: entry.file_type(),
        }
    }
}

impl HostClass for Dirent {
    const NAME: &'static str = "Dirent";

    fn build(spec: &mut ClassSpec<Self>) {
        spec.getter("name", |entry, _scope| Ok(entry.name.clone()));
        spec.getter("parentPath", |entry, _scope| Ok(entry.parent_path.clone()));
        spec.method("isFile", |entry, _scope, _args| Ok(entry.file_type == FileType::File));
        spec.method("isDirectory", |entry, _scope, _args| {
            Ok(entry.file_type == FileType::Directory)
        });
        spec.method("isSymbolicLink", |entry, _scope, _args| {
            Ok(entry.file_type == FileType::Symlink)
        });
        spec.method("isFIFO", |entry, _scope, _args| Ok(entry.file_type == FileType::Fifo));
        spec.method("isBlockDevice", |entry, _scope, _args| {
            Ok(entry.file_type == FileType::BlockDevice)
        });
        spec.method("isCharacterDevice", |entry, _scope, _args| {
            Ok(entry.file_type == FileType::CharacterDevice)
        });
        spec.method("isSocket", |entry, _scope, _args| Ok(entry.file_type == FileType::Socket));
    }
}

#[cfg(test)]
mod tests {
    use super::Dirent;
    use crate::{
        fs::{DirEntry, FileType},
        path::{Component, PathBuf},
    };

    #[test]
    fn dirent_reports_the_parent_of_a_nested_entry() {
        let entry = DirEntry::new(
            PathBuf::parse("/work/notes.txt").unwrap(),
            Component::new("notes.txt"),
            FileType::File,
            None,
        );

        let dirent = Dirent::new(&entry);

        assert_eq!(dirent.parent_path, "/work");
    }
}
