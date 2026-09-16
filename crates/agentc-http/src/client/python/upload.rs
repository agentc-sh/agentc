// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    cell::RefCell,
    error::Error as StdError,
    fmt::{Display, Formatter, Result as FmtResult},
};

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{
        errors::Error,
        handle::AsyncIter,
        host::exception::Raise,
    },
};
use bytes::Bytes;
use futures::{
    SinkExt,
    StreamExt,
    channel::mpsc::{self, Receiver, Sender},
};

use crate::client::python::exceptions::BodyConsumed;

#[derive(Debug)]
pub(crate) struct UploadFailed;

impl Display for UploadFailed {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter.write_str("guest request body failed")
    }
}

impl StdError for UploadFailed {}

pub(crate) struct Upload<B: ExecutorBackend> {
    chunks: AsyncIter<B, Bytes>,
    sender: Sender<Result<Bytes, UploadFailed>>,
}

impl<B: ExecutorBackend> Upload<B> {
    pub(crate) fn new(
        body: &RefCell<Option<AsyncIter<B, Bytes>>>,
    ) -> Result<(Self, Receiver<Result<Bytes, UploadFailed>>), Error> {
        let chunks = body.borrow_mut().take().ok_or_else(
            || {
                Raise::<B>::host(
                    BodyConsumed {
                        message: String::from("body already been consumed"),
                    },
                )
            },
        )?;
        let (sender, receiver) = mpsc::channel(0);

        Ok(
            (
                Self {
                    chunks,
                    sender,
                },
                receiver,
            ),
        )
    }

    pub(crate) async fn pump(mut self) -> Result<(), Error> {
        loop {
            match self.chunks.next().await {
                Some(Ok(chunk)) => {
                    if self.sender.send(Ok(chunk)).await.is_err() {
                        return Ok(());
                    }
                }
                Some(Err(error)) => {
                    let _ = self.sender.send(Err(UploadFailed)).await;

                    return Err(error);
                }
                None => return Ok(()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use agentc_executor_python::{
        executor::Executor,
        guestpy::{
            bundle::Bundle,
            handle::{AsyncIterable, ObjectProtocol},
            rustpython::RustPython,
        },
    };
    use bytes::Bytes;
    use futures::{StreamExt, future::join};

    use super::Upload;

    const SOURCE: &str = r#"
async def ordered():
    yield b"first "
    yield b"second"


async def failing():
    yield b"before"
    raise LookupError("upload failed")


def ordered_chunks():
    return ordered()


def failing_chunks():
    return failing()
"#;

    async fn executor() -> Executor<RustPython> {
        Executor::<RustPython>::builder("agentc_http_upload_test")
            .bundle(
                Bundle::single("agentc_http_upload_test", SOURCE).expect("bundle builds"),
            )
            .workers(1)
            .build()
            .await
            .expect("executor builds")
    }

    #[tokio::test]
    async fn upload_pumps_ordered_chunks_and_claims_the_iterator_once() {
        let executor = executor().await;

        assert_eq!(
            executor
                .execute(
                    |context| Box::pin(
                        async move {
                            let chunks = context
                                .module()
                                .function("ordered_chunks")?
                                .call::<_, AsyncIterable<RustPython, Bytes>>(())?;
                            let body = RefCell::new(Some(chunks.into_inner()));
                            let (upload, mut receiver) = Upload::new(&body)?;
                            let already_claimed = Upload::new(&body).is_err();
                            let (pumped, received) = join(
                                upload.pump(),
                                async move {
                                    let mut received = Vec::new();

                                    while let Some(chunk) = receiver.next().await {
                                        received.push(
                                            chunk.expect("chunk is valid").to_vec(),
                                        );
                                    }

                                    received
                                },
                            )
                            .await;

                            pumped?;

                            Ok((received, already_claimed))
                        },
                    ),
                )
                .await
                .expect("guest operation succeeds"),
            (
                vec![b"first ".to_vec(), b"second".to_vec()],
                true,
            ),
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn guest_iteration_failure_reaches_the_pump() {
        let executor = executor().await;

        assert!(
            executor
                .execute(
                    |context| Box::pin(
                        async move {
                            let chunks = context
                                .module()
                                .function("failing_chunks")?
                                .call::<_, AsyncIterable<RustPython, Bytes>>(())?;
                            let (upload, mut receiver) = Upload::new(
                                &RefCell::new(Some(chunks.into_inner())),
                            )?;
                            let (pumped, received_error) = join(
                                upload.pump(),
                                async move {
                                    let first = receiver.next().await;
                                    let second = receiver.next().await;

                                    assert_eq!(
                                        first
                                            .expect("first chunk exists")
                                            .expect("first chunk is valid"),
                                        Bytes::from_static(b"before"),
                                    );

                                    second.expect("failure marker exists").is_err()
                                },
                            )
                            .await;

                            Ok(pumped.is_err() && received_error)
                        },
                    ),
                )
                .await
                .expect("guest operation completes"),
        );

        executor.shutdown().await.expect("executor shuts down");
    }
}
