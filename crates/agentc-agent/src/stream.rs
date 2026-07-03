// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use futures::{Stream, future::AbortHandle, stream::Abortable};
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::{mpsc, watch};
use tokio_stream::wrappers::UnboundedReceiverStream;

#[derive(Debug, Clone)]
pub struct EventEmitter<T> {
    tx: mpsc::UnboundedSender<T>,
    paused_tx: watch::Sender<bool>,
    abort_handle: AbortHandle,
}

impl<T: Send + 'static> EventEmitter<T> {
    pub fn new_pair() -> (Self, EventStream<T>) {
        let (tx, rx) = mpsc::unbounded_channel::<T>();
        let (paused_tx, paused_rx) = watch::channel(false);
        let (abort_handle, abort_registration) = AbortHandle::new_pair();

        (
            EventEmitter { tx, paused_tx, abort_handle },
            EventStream {
                inner: Abortable::new(UnboundedReceiverStream::new(rx), abort_registration),
                paused_rx,
            },
        )
    }

    pub fn emit(&self, event: T) -> Result<(), mpsc::error::SendError<T>> {
        self.tx.send(event)
    }

    pub fn pause(&self) {
        let _ = self.paused_tx.send(true);
    }

    pub fn resume(&self) {
        let _ = self.paused_tx.send(false);
    }

    pub fn abort(&self) {
        self.abort_handle.abort();
    }

    pub fn is_paused(&self) -> bool {
        *self.paused_tx.borrow()
    }

    pub fn is_aborted(&self) -> bool {
        self.abort_handle.is_aborted()
    }
}

pub struct EventStream<T> {
    inner: Abortable<UnboundedReceiverStream<T>>,
    paused_rx: watch::Receiver<bool>,
}

impl<T> Stream for EventStream<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if *this.paused_rx.borrow() {
            cx.waker().wake_by_ref();

            return Poll::Pending;
        }

        Pin::new(&mut this.inner).poll_next(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn test_basic_emit_and_receive() {
        let (emitter, mut stream) = EventEmitter::new_pair();

        emitter.emit(1).unwrap();
        emitter.emit(2).unwrap();

        let mut collected = vec![];
        while let Some(item) = stream.next().await {
            collected.push(item);
            if collected.len() == 2 {
                break;
            }
        }

        assert_eq!(collected, vec![1, 2]);
    }

    #[tokio::test]
    async fn test_abort() {
        let (emitter, mut stream) = EventEmitter::new_pair();

        emitter.emit(1).unwrap();
        emitter.emit(2).unwrap();

        emitter.abort();

        let mut collected = vec![];
        while let Some(item) = stream.next().await {
            collected.push(item);
        }

        assert!(collected.len() <= 2);
        assert!(stream.next().await.is_none());
        assert!(emitter.is_aborted());
    }

    #[tokio::test]
    async fn test_multiple_tasks_publishing() {
        let (emitter, mut stream) = EventEmitter::new_pair();
        let emitter1 = emitter.clone();
        let emitter2 = emitter.clone();

        let task1 = tokio::spawn(async move {
            for i in 0..5 {
                emitter1.emit(i).unwrap();
                sleep(Duration::from_millis(10)).await;
            }
        });

        let task2 = tokio::spawn(async move {
            for i in 5..10 {
                emitter2.emit(i).unwrap();
                sleep(Duration::from_millis(15)).await;
            }
        });

        let mut collected = vec![];
        while collected.len() < 10 {
            if let Some(item) = stream.next().await {
                collected.push(item);
            }
        }

        task1.await.unwrap();
        task2.await.unwrap();

        collected.sort();
        assert_eq!(collected, (0..10).collect::<Vec<_>>());
    }

    #[tokio::test]
    async fn test_pause_resume_multiple_tasks() {
        let (emitter, mut stream) = EventEmitter::new_pair();
        let emitter1 = emitter.clone();
        let emitter2 = emitter.clone();

        emitter.pause();

        let task1 = tokio::spawn(async move {
            for i in 0..5 {
                emitter1.emit(i).unwrap();
                sleep(Duration::from_millis(10)).await;
            }
        });

        let task2 = tokio::spawn(async move {
            for i in 5..10 {
                emitter2.emit(i).unwrap();
                sleep(Duration::from_millis(15)).await;
            }
        });

        sleep(Duration::from_millis(100)).await;

        emitter.resume();

        let mut collected = vec![];
        while collected.len() < 10 {
            if let Some(item) = stream.next().await {
                collected.push(item);
            }
        }

        task1.await.unwrap();
        task2.await.unwrap();

        collected.sort();
        assert_eq!(collected, (0..10).collect::<Vec<_>>());
    }
}
