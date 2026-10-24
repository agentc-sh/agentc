// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::guestjs::{errors::Error, host_class};

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

#[host_class(rename_all = "camelCase", crate_path = agentc_executor_typescript::guestjs)]
impl Dirent {
    #[guestjs(get)]
    fn name(&self) -> Result<String, Error> {
        Ok(self.name.clone())
    }

    #[guestjs(get)]
    fn parent_path(&self) -> Result<String, Error> {
        Ok(self.parent_path.clone())
    }

    #[guestjs(method)]
    fn is_file(&self) -> Result<bool, Error> {
        Ok(self.file_type == FileType::File)
    }

    #[guestjs(method)]
    fn is_directory(&self) -> Result<bool, Error> {
        Ok(self.file_type == FileType::Directory)
    }

    #[guestjs(method)]
    fn is_symbolic_link(&self) -> Result<bool, Error> {
        Ok(self.file_type == FileType::Symlink)
    }

    #[guestjs(method, name = "isFIFO")]
    fn is_fifo(&self) -> Result<bool, Error> {
        Ok(self.file_type == FileType::Fifo)
    }

    #[guestjs(method)]
    fn is_block_device(&self) -> Result<bool, Error> {
        Ok(self.file_type == FileType::BlockDevice)
    }

    #[guestjs(method)]
    fn is_character_device(&self) -> Result<bool, Error> {
        Ok(self.file_type == FileType::CharacterDevice)
    }

    #[guestjs(method)]
    fn is_socket(&self) -> Result<bool, Error> {
        Ok(self.file_type == FileType::Socket)
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
