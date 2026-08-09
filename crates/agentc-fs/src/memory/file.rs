// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    cmp::min,
    io::{Error as IoError, ErrorKind, Result as IoResult, SeekFrom},
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
    task::{Context, Poll},
};

use tokio::io::{AsyncRead, AsyncSeek, AsyncWrite, ReadBuf};

pub struct MemoryFile {
    content: Arc<Mutex<Vec<u8>>>,
    position: u64,
}

impl MemoryFile {
    pub(crate) fn new(content: Arc<Mutex<Vec<u8>>>, position: u64) -> Self {
        MemoryFile { content, position }
    }

    fn lock_content(&self) -> IoResult<MutexGuard<'_, Vec<u8>>> {
        self.content
            .lock()
            .map_err(|_| IoError::new(ErrorKind::Other, "memory file lock is poisoned"))
    }
}

impl AsyncRead for MemoryFile {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        self.position += {
            let content = match self.lock_content() {
                Ok(content) => content,
                Err(error) => return Poll::Ready(Err(error)),
            };

            let available = content
                .get(self.position as usize..)
                .unwrap_or_default();

            let len = min(available.len(), buf.remaining());
            buf.put_slice(&available[..len]);
            len as u64
        };

        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for MemoryFile {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<IoResult<usize>> {
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

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncSeek for MemoryFile {
    fn start_seek(mut self: Pin<&mut Self>, position: SeekFrom) -> IoResult<()> {
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

    fn poll_complete(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<u64>> {
        Poll::Ready(Ok(self.position))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::SeekFrom,
        sync::{Arc, Mutex},
    };

    use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

    use crate::memory::file::MemoryFile;

    #[tokio::test]
    async fn memory_file_writes_seeks_and_reads() {
        let content = Arc::new(Mutex::new(Vec::new()));
        let mut file = MemoryFile::new(content.clone(), 0);

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

        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .await
            .unwrap();

        assert_eq!(bytes, b"hello agentc");
        assert_eq!(content.lock().unwrap().as_slice(), b"hello agentc");
    }
}
