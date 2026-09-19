// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::marker::PhantomData;

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{
        host_class,
        errors::Error,
        marshal::ToGuest,
        scope::Enter,
    },
};

use crate::{
    fs::{DirEntry, FileType},
    python::stat::Stat,
};

pub(crate) struct EntryType(FileType);

impl EntryType {
    pub(crate) fn as_inner(&self) -> &FileType {
        &self.0
    }

    pub(crate) fn as_str(&self) -> &'static str {
        match self.as_inner() {
            FileType::File => "file",
            FileType::Directory => "directory",
            FileType::Symlink => "symlink",
            FileType::Fifo => "fifo",
            FileType::Socket => "socket",
            FileType::BlockDevice => "block_device",
            FileType::CharacterDevice => "character_device",
            FileType::Other => "other",
        }
    }
}

impl From<FileType> for EntryType {
    fn from(file_type: FileType) -> Self {
        Self(file_type)
    }
}

impl<B: ExecutorBackend> ToGuest<B> for EntryType {
    fn to_guest<'py>(self, enter: &Enter<'py, B>) -> Result<B::Value<'py>, Error> {
        self.as_str()
            .to_guest(enter)
    }
}

pub struct Entry<B: ExecutorBackend> {
    entry: DirEntry,
    _marker: PhantomData<fn() -> B>,
}

#[host_class(backend = B, crate_path = agentc_executor_python::guestpy)]
impl<B: ExecutorBackend> Entry<B> {
    #[guestpy(get)]
    fn name(&self) -> Result<String, Error> {
        Ok(self.entry.file_name().to_string_lossy())
    }

    #[guestpy(get)]
    fn path(&self) -> Result<String, Error> {
        Ok(self.entry.path().to_string_lossy())
    }

    #[guestpy(get, name = "type")]
    fn r#type(&self) -> Result<EntryType, Error> {
        Ok(EntryType::from(self.entry.file_type()))
    }

    #[guestpy(get)]
    fn is_file(&self) -> Result<bool, Error> {
        Ok(self.entry.file_type() == FileType::File)
    }

    #[guestpy(get)]
    fn is_dir(&self) -> Result<bool, Error> {
        Ok(self.entry.file_type() == FileType::Directory)
    }

    #[guestpy(get)]
    fn is_symlink(&self) -> Result<bool, Error> {
        Ok(self.entry.file_type() == FileType::Symlink)
    }

    #[guestpy(get)]
    fn stat(&self) -> Result<Option<Stat<B>>, Error> {
        Ok(self.entry.metadata().cloned().map(Stat::<B>::from))
    }
}

impl<B: ExecutorBackend> From<DirEntry> for Entry<B> {
    fn from(entry: DirEntry) -> Self {
        Self { entry, _marker: PhantomData }
    }
}

#[cfg(test)]
mod tests {
    use agentc_executor_python::guestpy::rustpython::RustPython;

    use crate::{
        fs::{DirEntry, FileType, Metadata, Permissions},
        path::{Component, PathBuf},
    };

    use super::Entry;

    #[test]
    fn entry_projects_the_core_directory_entry() {
        let entry = Entry::<RustPython>::from(DirEntry::new(
            PathBuf::parse("/notes.txt").unwrap(),
            Component::new("notes.txt"),
            FileType::File,
            Metadata::file(5, Permissions::new(0o644)),
        ));

        assert_eq!(entry.name().unwrap(), "notes.txt");
        assert_eq!(entry.path().unwrap(), "/notes.txt");
        assert!(entry.is_file().unwrap());
        assert!(!entry.is_dir().unwrap());
        assert!(!entry.is_symlink().unwrap());
        assert!(entry.stat().unwrap().is_some());
    }

    #[test]
    fn entry_preserves_absent_metadata() {
        let entry = Entry::<RustPython>::from(DirEntry::new(
            PathBuf::parse("/socket").unwrap(),
            Component::new("socket"),
            FileType::Socket,
            None,
        ));

        assert!(entry.stat().unwrap().is_none());
    }
}
