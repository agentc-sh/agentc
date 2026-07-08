// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use futures::{Stream, future::AbortHandle, stream::Abortable};
use std::{
    future::Future,
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

pub struct RunStream<E> {
    fut: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
    inner: EventStream<E>,
}

impl<E> RunStream<E> {
    /// Create a self-driving run stream from the run future and the event
    /// receiver it emits into. Polling the stream advances the run future;
    /// dropping the stream drops the future and cancels the run.
    pub fn new(fut: Pin<Box<dyn Future<Output = ()> + Send>>, inner: EventStream<E>) -> Self {
        Self { fut: Some(fut), inner }
    }

    pub fn builder() -> RunStreamBuilder<E> {
        RunStreamBuilder::new()
    }
}

impl<E> Stream for RunStream<E> {
    type Item = E;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            if let Poll::Ready(event) = Pin::new(&mut this.inner).poll_next(cx) {
                return Poll::Ready(event);
            }

            match this.fut.as_mut() {
                Some(fut) => match fut.as_mut().poll(cx) {
                    Poll::Ready(()) => {
                        this.fut = None;
                        continue;
                    },
                    Poll::Pending => return Poll::Pending,
                },
                None => return Poll::Pending,
            }
        }
    }
}

pub struct RunStreamBuilder<E> {
    fut: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
    inner: Option<EventStream<E>>,
}

impl<E> RunStreamBuilder<E> {
    pub fn new() -> Self {
        Self {
            fut: None,
            inner: None
        }
    }

    pub fn with_future<Fut>(mut self, fut: Fut) -> Self
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.fut = Some(Box::pin(fut));
        self
    }

    pub fn with_inner(mut self, inner: EventStream<E>) -> Self {
        self.inner = Some(inner);
        self
    }

    pub fn build(self) -> RunStream<E> {
        RunStream::new(
            self.fut.expect("RunStreamBuilder: future not set"),
            self.inner.expect("RunStreamBuilder: inner stream not set")
        )
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

    #[tokio::test]
    async fn run_stream_delivers_all_events_then_ends() {
        let (emitter, inner) = EventEmitter::new_pair();

        let fut = Box::pin(async move {
            emitter.emit(1).unwrap();
            emitter.emit(2).unwrap();
            emitter.emit(3).unwrap();
        });

        let collected = RunStream::new(fut, inner).collect::<Vec<_>>().await;
        assert_eq!(collected, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn run_stream_cancels_run_when_dropped() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };

        struct DropFlag(Arc<AtomicBool>);
        impl Drop for DropFlag {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        let dropped = Arc::new(AtomicBool::new(false));
        let (emitter, inner) = EventEmitter::new_pair();

        let guard = DropFlag(dropped.clone());
        let fut = Box::pin(async move {
            let _guard = guard;

            emitter.emit(1).unwrap();

            std::future::pending::<()>().await;
        });

        let mut stream = RunStream::new(fut, inner);

        assert_eq!(stream.next().await, Some(1));

        drop(stream);

        assert!(dropped.load(Ordering::SeqCst), "run future was not dropped on stream drop");
    }
}
