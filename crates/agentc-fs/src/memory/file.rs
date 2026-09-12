// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    cmp::min,
    io::{Error as IoError, ErrorKind, Result as IoResult, SeekFrom},
    sync::{Arc, Mutex, MutexGuard},
    task::{Context, Poll},
};

use async_trait::async_trait;
use tokio::io::ReadBuf;

use crate::{backend::FileHandle, errors::Error, path::PathBuf};

pub struct MemoryFile {
    path: PathBuf,
    content: Arc<Mutex<Vec<u8>>>,
    position: u64,
    writable: bool,
}

impl MemoryFile {
    pub(crate) fn new(
        path: PathBuf,
        content: Arc<Mutex<Vec<u8>>>,
        position: u64,
        writable: bool,
    ) -> Self {
        MemoryFile { path, content, position, writable }
    }

    fn lock_content(&self) -> IoResult<MutexGuard<'_, Vec<u8>>> {
        self.content
            .lock()
            .map_err(|_| IoError::new(ErrorKind::Other, "memory file lock is poisoned"))
    }
}

#[async_trait]
impl FileHandle for MemoryFile {
    fn poll_read(&mut self, _cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<IoResult<()>> {
        let content = match self.lock_content() {
            Ok(content) => content,
            Err(error) => return Poll::Ready(Err(error)),
        };

        let available = content
            .get(self.position as usize..)
            .unwrap_or_default();

        let len = min(available.len(), buf.remaining());
        buf.put_slice(&available[..len]);
        drop(content);

        self.position += len as u64;

        Poll::Ready(Ok(()))
    }

    fn poll_write(&mut self, _cx: &mut Context<'_>, bytes: &[u8]) -> Poll<IoResult<usize>> {
        let position = self.position as usize;

        {
            let mut content = match self.lock_content() {
                Ok(content) => content,
                Err(error) => return Poll::Ready(Err(error)),
            };
            let end = position + bytes.len();

            if content.len() < position {
                content.resize(position, 0);
            }

            if content.len() < end {
                content.resize(end, 0);
            }

            content[position..end].copy_from_slice(bytes);
        }

        self.position += bytes.len() as u64;
        Poll::Ready(Ok(bytes.len()))
    }

    fn poll_flush(&mut self, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(&mut self, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Ok(()))
    }

    fn start_seek(&mut self, position: SeekFrom) -> IoResult<()> {
        let position = match position {
            SeekFrom::Start(position) => position as i64,
            SeekFrom::End(offset) => self.lock_content()?.len() as i64 + offset,
            SeekFrom::Current(offset) => self.position as i64 + offset,
        };

        if position < 0 {
            return Err(IoError::new(ErrorKind::InvalidInput, "cannot seek before start of file"));
        }

        self.position = position as u64;
        Ok(())
    }

    fn poll_seek(&mut self, _cx: &mut Context<'_>) -> Poll<IoResult<u64>> {
        Poll::Ready(Ok(self.position))
    }

    async fn set_len(&mut self, len: u64) -> Result<(), Error> {
        if !self.writable {
            return Err(Error::permission_denied(self.path.clone()));
        }

        self.lock_content()
            .map_err(|_| Error::unexpected("memory file lock is poisoned"))?
            .resize(len as usize, 0);

        Ok(())
    }

    async fn sync_all(&mut self) -> Result<(), Error> {
        Ok(())
    }

    async fn sync_data(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::SeekFrom,
        sync::{Arc, Mutex},
    };

    use tokio::io::AsyncSeekExt;

    use crate::{
        backend::FileHandle, errors::Error, fs::File, memory::file::MemoryFile, path::PathBuf,
    };

    #[tokio::test]
    async fn memory_file_writes_seeks_and_reads() {
        let content = Arc::new(Mutex::new(Vec::new()));
        let mut file = File::new(Box::new(MemoryFile::new(
            PathBuf::parse("/notes.txt").unwrap(),
            content.clone(),
            0,
            true,
        )));

        file.write_all(b"hello world")
            .await
            .unwrap();
        file.seek(SeekFrom::Start(6))
            .await
            .unwrap();
        file.write_all(b"agentc").await.unwrap();
        file.seek(SeekFrom::Start(0))
            .await
            .unwrap();

        let bytes = file.read_to_end().await.unwrap();

        assert_eq!(bytes, b"hello agentc");
        assert_eq!(content.lock().unwrap().as_slice(), b"hello agentc");
    }

    #[tokio::test]
    async fn memory_file_set_len_shrinks_and_zero_fills() {
        let content = Arc::new(Mutex::new(b"hello world".to_vec()));
        let mut file =
            MemoryFile::new(PathBuf::parse("/notes.txt").unwrap(), content.clone(), 0, true);

        file.set_len(5).await.unwrap();
        assert_eq!(content.lock().unwrap().as_slice(), b"hello");

        file.set_len(8).await.unwrap();
        assert_eq!(content.lock().unwrap().as_slice(), b"hello\0\0\0");
    }

    #[tokio::test]
    async fn memory_file_set_len_leaves_the_offset_alone() {
        let content = Arc::new(Mutex::new(b"hello world".to_vec()));
        let mut file = File::new(Box::new(MemoryFile::new(
            PathBuf::parse("/notes.txt").unwrap(),
            content,
            11,
            true,
        )));

        file.set_len(5).await.unwrap();

        assert_eq!(file.read_to_end().await.unwrap(), b"");
    }

    #[tokio::test]
    async fn memory_file_set_len_is_denied_on_a_read_only_handle() {
        assert!(matches!(
            MemoryFile::new(
                PathBuf::parse("/notes.txt").unwrap(),
                Arc::new(Mutex::new(b"hello world".to_vec())),
                0,
                false,
            )
            .set_len(5)
            .await,
            Err(Error::PermissionDenied(path)) if path.to_string_lossy() == "/notes.txt"
        ));
    }
}
