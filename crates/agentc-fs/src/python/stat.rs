// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::marker::PhantomData;

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{errors::Error, host_class},
};

use crate::{
    fs::{FileType, Metadata},
    python::{entry::EntryType, timestamp::Timestamp},
};

pub struct Stat<B: ExecutorBackend> {
    metadata: Metadata,
    _marker: PhantomData<fn() -> B>,
}

#[host_class(backend = B, crate_path = agentc_executor_python::guestpy)]
impl<B: ExecutorBackend> Stat<B> {
    #[guestpy(get, name = "type")]
    fn r#type(&self) -> Result<EntryType, Error> {
        Ok(EntryType::from(self.metadata.file_type()))
    }

    #[guestpy(get)]
    fn is_file(&self) -> Result<bool, Error> {
        Ok(self.metadata.file_type() == FileType::File)
    }

    #[guestpy(get)]
    fn is_dir(&self) -> Result<bool, Error> {
        Ok(self.metadata.file_type() == FileType::Directory)
    }

    #[guestpy(get)]
    fn is_symlink(&self) -> Result<bool, Error> {
        Ok(self.metadata.file_type() == FileType::Symlink)
    }

    #[guestpy(get)]
    fn size(&self) -> Result<u64, Error> {
        Ok(self.metadata.len())
    }

    #[guestpy(get)]
    fn mode(&self) -> Result<u32, Error> {
        Ok(self.metadata.permissions().mode())
    }

    #[guestpy(get)]
    fn is_readonly(&self) -> Result<bool, Error> {
        Ok(self
            .metadata
            .permissions()
            .is_readonly())
    }

    #[guestpy(get)]
    fn accessed(&self) -> Result<Option<Timestamp>, Error> {
        Ok(self
            .metadata
            .accessed()
            .map(Timestamp::from))
    }

    #[guestpy(get)]
    fn modified(&self) -> Result<Option<Timestamp>, Error> {
        Ok(self
            .metadata
            .modified()
            .map(Timestamp::from))
    }

    #[guestpy(get)]
    fn created(&self) -> Result<Option<Timestamp>, Error> {
        Ok(self
            .metadata
            .created()
            .map(Timestamp::from))
    }

    #[guestpy(get)]
    fn changed(&self) -> Result<Option<Timestamp>, Error> {
        Ok(self
            .metadata
            .changed()
            .map(Timestamp::from))
    }

    #[guestpy(get)]
    fn dev(&self) -> Result<u64, Error> {
        Ok(self.metadata.dev())
    }

    #[guestpy(get)]
    fn ino(&self) -> Result<u64, Error> {
        Ok(self.metadata.ino())
    }

    #[guestpy(get)]
    fn nlink(&self) -> Result<u64, Error> {
        Ok(self.metadata.nlink())
    }

    #[guestpy(get)]
    fn uid(&self) -> Result<u32, Error> {
        Ok(self.metadata.uid())
    }

    #[guestpy(get)]
    fn gid(&self) -> Result<u32, Error> {
        Ok(self.metadata.gid())
    }

    #[guestpy(get)]
    fn rdev(&self) -> Result<u64, Error> {
        Ok(self.metadata.rdev())
    }

    #[guestpy(get)]
    fn blksize(&self) -> Result<u64, Error> {
        Ok(self.metadata.blksize())
    }

    #[guestpy(get)]
    fn blocks(&self) -> Result<u64, Error> {
        Ok(self.metadata.blocks())
    }
}

impl<B: ExecutorBackend> From<Metadata> for Stat<B> {
    fn from(metadata: Metadata) -> Self {
        Self { metadata, _marker: PhantomData }
    }
}

#[cfg(test)]
mod tests {
    use agentc_executor_python::guestpy::rustpython::RustPython;

    use crate::fs::{Metadata, Permissions};

    use super::Stat;

    #[test]
    fn stat_projects_core_metadata() {
        let stat = Stat::<RustPython>::from(
            Metadata::file(513, Permissions::new(0o10444))
                .with_dev(1)
                .with_ino(2)
                .with_nlink(3)
                .with_uid(4)
                .with_gid(5)
                .with_rdev(6)
                .with_blksize(7)
                .with_blocks(8),
        );

        assert!(stat.is_file().unwrap());
        assert!(!stat.is_dir().unwrap());
        assert!(!stat.is_symlink().unwrap());
        assert_eq!(stat.size().unwrap(), 513);
        assert_eq!(stat.mode().unwrap(), 0o444);
        assert!(stat.is_readonly().unwrap());
        assert_eq!(stat.dev().unwrap(), 1);
        assert_eq!(stat.ino().unwrap(), 2);
        assert_eq!(stat.nlink().unwrap(), 3);
        assert_eq!(stat.uid().unwrap(), 4);
        assert_eq!(stat.gid().unwrap(), 5);
        assert_eq!(stat.rdev().unwrap(), 6);
        assert_eq!(stat.blksize().unwrap(), 7);
        assert_eq!(stat.blocks().unwrap(), 8);
    }
}
