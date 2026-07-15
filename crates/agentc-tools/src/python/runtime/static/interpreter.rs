// SPDX-FileCopyrightText: 2026 Timothy Pogue
//
// SPDX-License-Identifier: LicenseRef-Proprietary

use pyo3::Python;
use serde_json::Value;
use std::{
    sync::mpsc::{self, RecvTimeoutError},
    thread::{self, JoinHandle},
    time::Duration,
};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::python::runtime::{errors::RuntimeError, protocol::Command};

use super::context::InterpreterContext;

struct InterpreterMessage {
    command: Command,
    tx: oneshot::Sender<Result<Value, RuntimeError>>,
}

struct StaticInterpreter {
    channel_size: usize,
    shutdown: CancellationToken,
}

impl StaticInterpreter {
    fn new(channel_size: usize, shutdown: CancellationToken) -> Self {
        Self { channel_size, shutdown }
    }

    fn spawn(self) -> Result<(mpsc::SyncSender<InterpreterMessage>, JoinHandle<()>), RuntimeError> {
        let (ready_tx, ready_rx) = mpsc::channel();

        let handle = thread::Builder::new()
            .spawn(move || self.run(ready_tx))
            .map_err(RuntimeError::io)?;

        let msg_tx = ready_rx
            .recv()
            .map_err(|_| RuntimeError::worker_closed())??;

        Ok((msg_tx, handle))
    }

    fn run(
        self,
        ready_tx: mpsc::Sender<Result<mpsc::SyncSender<InterpreterMessage>, RuntimeError>>,
    ) {
        Python::initialize();

        let (msg_tx, msg_rx) = mpsc::sync_channel(self.channel_size);

        if ready_tx.send(Ok(msg_tx)).is_err() {
            return;
        }

        let ctx = Python::attach(|py| InterpreterContext::new(py));

        loop {
            match msg_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(InterpreterMessage { command, tx: reply }) => {
                    let _ = reply.send(Python::attach(|py| ctx.dispatch(py, command)));
                }
                Err(RecvTimeoutError::Timeout) => {
                    if self.shutdown.is_cancelled() {
                        break;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }
}

pub(super) struct StaticRuntimeWorker {
    msg_tx: mpsc::SyncSender<InterpreterMessage>,
    handle: JoinHandle<()>,
}

impl StaticRuntimeWorker {
    pub(super) fn new(
        channel_size: usize,
        shutdown: CancellationToken,
    ) -> Result<Self, RuntimeError> {
        let (tx, handle) = StaticInterpreter::new(channel_size, shutdown).spawn()?;
        Ok(Self { msg_tx: tx, handle })
    }

    pub(super) fn close(self) -> Result<(), RuntimeError> {
        drop(self.msg_tx);

        self.handle.join().map_err(|err| {
            err.downcast::<RuntimeError>()
                .map(|e| *e)
                .unwrap_or(RuntimeError::worker_closed())
        })
    }

    pub(super) fn send_command(
        &self,
        command: Command,
    ) -> oneshot::Receiver<Result<Value, RuntimeError>> {
        let (reply_tx, reply_rx) = oneshot::channel();

        if let Err(e) = self
            .msg_tx
            .try_send(InterpreterMessage { command, tx: reply_tx })
        {
            let (err, msg) = match e {
                mpsc::TrySendError::Full(m) => (RuntimeError::worker_busy(), m),
                mpsc::TrySendError::Disconnected(m) => (RuntimeError::worker_closed(), m),
            };

            let _ = msg.tx.send(Err(err));
        }

        reply_rx
    }
}
