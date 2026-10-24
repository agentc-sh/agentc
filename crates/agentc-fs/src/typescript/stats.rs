// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{marker::PhantomData, time::UNIX_EPOCH};

use agentc_executor_typescript::{
    backend::ExecutorBackend,
    guestjs::{errors::Error, host_class},
};

use crate::{
    fs::{FileType, Metadata},
    typescript::timestamp::Timestamp,
};

pub struct Stats<B: ExecutorBackend> {
    metadata: Metadata,
    backend: PhantomData<fn() -> B>,
}

impl<B: ExecutorBackend> Stats<B> {
    pub fn new(metadata: Metadata) -> Self {
        Self {
            metadata,
            backend: PhantomData,
        }
    }

    fn accessed(&self) -> Timestamp {
        Timestamp::from(
            self.metadata
                .accessed()
                .unwrap_or(UNIX_EPOCH)
        )
    }

    fn modified(&self) -> Timestamp {
        Timestamp::from(
            self.metadata
                .modified()
                .unwrap_or(UNIX_EPOCH)
        )
    }

    fn changed(&self) -> Timestamp {
        Timestamp::from(
            self.metadata
                .changed()
                .or_else(|| self.metadata.modified())
                .unwrap_or(UNIX_EPOCH)
        )
    }

    fn created(&self) -> Timestamp {
        Timestamp::from(
            self.metadata
                .created()
                .unwrap_or(UNIX_EPOCH)
        )
    }
}

#[host_class(backend = B, rename_all = "camelCase", crate_path = agentc_executor_typescript::guestjs)]
impl<B: ExecutorBackend> Stats<B> {
    #[guestjs(get)]
    fn dev(&self) -> Result<u64, Error> {
        Ok(self.metadata.dev())
    }

    #[guestjs(get)]
    fn ino(&self) -> Result<u64, Error> {
        Ok(self.metadata.ino())
    }

    #[guestjs(get)]
    fn mode(&self) -> Result<u32, Error> {
        let file_type = match self.metadata.file_type() {
            FileType::File => 0o100000,
            FileType::Directory => 0o40000,
            FileType::Symlink => 0o120000,
            FileType::Fifo => 0o10000,
            FileType::Socket => 0o140000,
            FileType::BlockDevice => 0o60000,
            FileType::CharacterDevice => 0o20000,
            FileType::Other => 0,
        };

        Ok(file_type | self.metadata.permissions().mode())
    }

    #[guestjs(get)]
    fn nlink(&self) -> Result<u64, Error> {
        Ok(self.metadata.nlink())
    }

    #[guestjs(get)]
    fn uid(&self) -> Result<u32, Error> {
        Ok(self.metadata.uid())
    }

    #[guestjs(get)]
    fn gid(&self) -> Result<u32, Error> {
        Ok(self.metadata.gid())
    }

    #[guestjs(get)]
    fn rdev(&self) -> Result<u64, Error> {
        Ok(self.metadata.rdev())
    }

    #[guestjs(get)]
    fn size(&self) -> Result<f64, Error> {
        Ok(self.metadata.len() as f64)
    }

    #[guestjs(get)]
    fn blksize(&self) -> Result<u64, Error> {
        Ok(self.metadata.blksize())
    }

    #[guestjs(get)]
    fn blocks(&self) -> Result<u64, Error> {
        Ok(self.metadata.blocks())
    }

    #[guestjs(get)]
    fn atime_ms(&self) -> Result<f64, Error> {
        Ok(self.accessed().millis())
    }

    #[guestjs(get)]
    fn mtime_ms(&self) -> Result<f64, Error> {
        Ok(self.modified().millis())
    }

    #[guestjs(get)]
    fn ctime_ms(&self) -> Result<f64, Error> {
        Ok(self.changed().millis())
    }

    #[guestjs(get)]
    fn birthtime_ms(&self) -> Result<f64, Error> {
        Ok(self.created().millis())
    }

    #[guestjs(get)]
    fn atime(&self) -> Result<Timestamp, Error> {
        Ok(self.accessed())
    }

    #[guestjs(get)]
    fn mtime(&self) -> Result<Timestamp, Error> {
        Ok(self.modified())
    }

    #[guestjs(get)]
    fn ctime(&self) -> Result<Timestamp, Error> {
        Ok(self.changed())
    }

    #[guestjs(get)]
    fn birthtime(&self) -> Result<Timestamp, Error> {
        Ok(self.created())
    }

    #[guestjs(method)]
    fn is_file(&self) -> Result<bool, Error> {
        Ok(self.metadata.file_type() == FileType::File)
    }

    #[guestjs(method)]
    fn is_directory(&self) -> Result<bool, Error> {
        Ok(self.metadata.file_type() == FileType::Directory)
    }

    #[guestjs(method)]
    fn is_dir(&self) -> Result<bool, Error> {
        Ok(self.metadata.file_type() == FileType::Directory)
    }

    #[guestjs(method)]
    fn is_symbolic_link(&self) -> Result<bool, Error> {
        Ok(self.metadata.file_type() == FileType::Symlink)
    }

    #[guestjs(method)]
    fn is_symlink(&self) -> Result<bool, Error> {
        Ok(self.metadata.file_type() == FileType::Symlink)
    }

    #[guestjs(method, name = "isFIFO")]
    fn is_fifo(&self) -> Result<bool, Error> {
        Ok(self.metadata.file_type() == FileType::Fifo)
    }

    #[guestjs(method)]
    fn is_block_device(&self) -> Result<bool, Error> {
        Ok(self.metadata.file_type() == FileType::BlockDevice)
    }

    #[guestjs(method)]
    fn is_character_device(&self) -> Result<bool, Error> {
        Ok(self.metadata.file_type() == FileType::CharacterDevice)
    }

    #[guestjs(method)]
    fn is_socket(&self) -> Result<bool, Error> {
        Ok(self.metadata.file_type() == FileType::Socket)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use agentc_executor_typescript::guestjs::quickjs::QuickJs;

    use super::Stats;
    use crate::fs::{FileType, Metadata, Permissions};

    #[test]
    fn mode_recombines_the_file_type_and_permission_bits() {
        let stats =
            Stats::<QuickJs>::new(Metadata::new(FileType::Directory, 0, Permissions::new(0o755)));

        assert_eq!(stats.mode().unwrap(), 0o40755);
    }

    #[test]
    fn millis_reports_zero_for_a_missing_timestamp() {
        let stats =
            Stats::<QuickJs>::new(Metadata::new(FileType::File, 0, Permissions::new(0o644)));

        assert_eq!(stats.accessed().millis(), 0.0);
    }

    #[test]
    fn changed_falls_back_to_modified() {
        let modified = SystemTime::UNIX_EPOCH + Duration::from_millis(1234);
        let stats = Stats::<QuickJs>::new(
            Metadata::new(FileType::File, 0, Permissions::new(0o644)).with_modified(modified),
        );

        assert_eq!(stats.changed().millis(), 1234.0);
    }
}
