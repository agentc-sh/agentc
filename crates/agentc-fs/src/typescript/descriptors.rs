// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    cell::RefCell,
    collections::HashMap,
    io::SeekFrom,
    rc::Rc,
};

use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::{
    errors::Error,
    fs::File,
    path::PathBuf,
};

pub struct Session {
    file: File,
}

impl Session {
    pub fn new(file: File) -> Self {
        Self { file }
    }

    async fn seek(&mut self, position: SeekFrom) -> Result<u64, Error> {
        self.file
            .seek(position)
            .await
            .map_err(|error| Error::sourced_unexpected("failed to seek file", error))
    }

    pub async fn read(&mut self, len: usize, position: Option<u64>) -> Result<Vec<u8>, Error> {
        let original = match position {
            Some(position) => {
                let original = self.seek(SeekFrom::Current(0)).await?;

                self.seek(SeekFrom::Start(position)).await?;

                Some(original)
            }
            None => None,
        };

        let mut bytes = vec![0; len];
        let mut filled = 0;

        while filled < bytes.len() {
            let read = self
                .file
                .read(&mut bytes[filled..])
                .await
                .map_err(|error| Error::sourced_unexpected("failed to read file", error))?;

            if read == 0 {
                break;
            }

            filled += read;
        }

        let result = {
            bytes.truncate(filled);

            Ok(bytes)
        };

        if let Some(original) = original {
            self.seek(SeekFrom::Start(original)).await?;
        }

        result
    }

    pub async fn write(&mut self, bytes: Vec<u8>, position: Option<u64>) -> Result<usize, Error> {
        let original = match position {
            Some(position) => {
                let original = self.seek(SeekFrom::Current(0)).await?;

                self.seek(SeekFrom::Start(position)).await?;

                Some(original)
            }
            None => None,
        };

        let len = bytes.len();
        let result = self
            .file
            .write_all(bytes)
            .await
            .map(|()| len);

        if let Some(original) = original {
            self.seek(SeekFrom::Start(original)).await?;
        }

        result
    }

    pub async fn read_to_end(&mut self) -> Result<Vec<u8>, Error> {
        self.file.read_to_end().await
    }

    pub async fn truncate(&mut self, len: u64) -> Result<(), Error> {
        self.file.set_len(len).await
    }

    pub async fn sync_all(&mut self) -> Result<(), Error> {
        self.file.sync_all().await
    }

    pub async fn sync_data(&mut self) -> Result<(), Error> {
        self.file.sync_data().await
    }
}

#[derive(Clone)]
pub struct Descriptors {
    inner: Rc<RefCell<DescriptorTable>>,
}

struct DescriptorTable {
    next: u32,
    entries: HashMap<u32, Descriptor>,
}

struct Descriptor {
    path: PathBuf,
    session: Option<Session>,
}

impl Descriptors {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(DescriptorTable {
                next: 3,
                entries: HashMap::new(),
            })),
        }
    }

    pub fn insert(&self, path: PathBuf, file: File) -> u32 {
        let mut table = self.inner.borrow_mut();
        let fd = table.next;

        table.next += 1;
        table.entries.insert(
            fd,
            Descriptor {
                path,
                session: Some(Session::new(file)),
            },
        );

        fd
    }

    pub fn path(&self, fd: u32) -> Result<PathBuf, Error> {
        self.inner
            .borrow()
            .entries
            .get(&fd)
            .map(|descriptor| descriptor.path.clone())
            .ok_or_else(|| Error::unexpected(format!("agentc:fs: bad file descriptor {fd}")))
    }

    pub fn close(&self, fd: u32) -> Result<(), Error> {
        self.inner
            .borrow_mut()
            .entries
            .remove(&fd)
            .map(|_| ())
            .ok_or_else(|| Error::unexpected(format!("agentc:fs: bad file descriptor {fd}")))
    }

    pub fn take(&self, fd: u32) -> Result<Session, Error> {
        let mut table = self.inner.borrow_mut();
        let descriptor = table
            .entries
            .get_mut(&fd)
            .ok_or_else(|| Error::unexpected(format!("agentc:fs: bad file descriptor {fd}")))?;

        descriptor
            .session
            .take()
            .ok_or_else(|| Error::unexpected(format!("agentc:fs: file descriptor {fd} is busy")))
    }

    pub fn restore(&self, fd: u32, session: Session) {
        if let Some(descriptor) = self.inner.borrow_mut().entries.get_mut(&fd)
            && descriptor.session.is_none()
        {
            descriptor.session = Some(session);
        }
    }

    pub fn lease(&self, fd: u32) -> Result<Lease, Error> {
        Ok(Lease {
            descriptors: self.clone(),
            fd,
            session: Some(self.take(fd)?),
        })
    }
}

pub struct Lease {
    descriptors: Descriptors,
    fd: u32,
    session: Option<Session>,
}

impl Lease {
    pub fn session(&mut self) -> Result<&mut Session, Error> {
        self.session
            .as_mut()
            .ok_or_else(|| Error::unexpected("agentc:fs: descriptor lease is empty"))
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        if let Some(session) = self.session.take() {
            self.descriptors.restore(self.fd, session);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::SeekFrom;

    use tokio::io::AsyncSeekExt;

    use super::{Descriptors, Session};
    use crate::fs::Fs;

    #[tokio::test]
    async fn insert_starts_at_three_and_increments() {
        let root = Fs::memory().root();
        let file_one = root
            .options()
            .write(true)
            .create(true)
            .open("/one.txt")
            .await
            .unwrap();
        let file_two = root
            .options()
            .write(true)
            .create(true)
            .open("/two.txt")
            .await
            .unwrap();
        let descriptors = Descriptors::new();

        assert_eq!(descriptors.insert(root.resolve("/one.txt").unwrap(), file_one), 3);
        assert_eq!(descriptors.insert(root.resolve("/two.txt").unwrap(), file_two), 4);
    }

    #[tokio::test]
    async fn taking_a_busy_descriptor_reports_a_clear_error() {
        let root = Fs::memory().root();
        let file = root
            .options()
            .write(true)
            .create(true)
            .open("/busy.txt")
            .await
            .unwrap();
        let descriptors = Descriptors::new();
        let fd = descriptors.insert(root.resolve("/busy.txt").unwrap(), file);
        let _session = descriptors.take(fd).unwrap();

        assert!(matches!(
            descriptors.take(fd),
            Err(crate::errors::Error::Unexpected {
                message,
                ..
            }) if message == format!("agentc:fs: file descriptor {fd} is busy")
        ));
    }

    #[tokio::test]
    async fn a_dropped_lease_returns_the_session() {
        let root = Fs::memory().root();
        let file = root
            .options()
            .write(true)
            .create(true)
            .open("/lease.txt")
            .await
            .unwrap();
        let descriptors = Descriptors::new();
        let fd = descriptors.insert(root.resolve("/lease.txt").unwrap(), file);

        {
            let _lease = descriptors.lease(fd).unwrap();
        }

        assert!(descriptors.take(fd).is_ok());
    }

    #[tokio::test]
    async fn positional_read_restores_the_cursor() {
        let root = Fs::memory().root();
        let mut file = root
            .options()
            .read(true)
            .write(true)
            .create(true)
            .open("/cursor.txt")
            .await
            .unwrap();

        file.write_all(b"abcdef").await.unwrap();
        file.seek(SeekFrom::Start(4)).await.unwrap();

        let mut session = Session::new(file);

        assert_eq!(session.read(2, Some(1)).await.unwrap(), b"bc");
        assert_eq!(session.read(2, None).await.unwrap(), b"ef");
    }
}
