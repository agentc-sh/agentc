// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, future::Future, rc::Rc};

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{
        errors::Error,
        handle::{Instance, Object},
        host::exception::{ExceptionClass, Raise},
        host_class,
    },
};

use crate::client::{
    errors::HttpClientError,
    python::{
        exchange::Exchange,
        request::Request,
        response::{Response, ResponseBodyHandle},
        upload::Upload,
    },
    request::HttpRequestBuilder,
    response::HttpResponse,
};

enum SendError {
    Guest(Error),
    Host(HttpClientError),
}

enum Pending<B: ExecutorBackend> {
    Ready {
        request: HttpRequestBuilder,
        origin: Instance<B, Request<B>>,
        upload: Option<Upload<B>>,
    },
    Sending,
    Entered(ResponseBodyHandle),
    Done,
}

pub struct PendingResponse<B: ExecutorBackend> {
    state: Rc<RefCell<Pending<B>>>,
}

impl<B: ExecutorBackend> PendingResponse<B> {
    pub(crate) fn new(
        request: HttpRequestBuilder,
        origin: Instance<B, Request<B>>,
        upload: Option<Upload<B>>,
    ) -> Self {
        Self {
            state: Rc::new(
                RefCell::new(
                    Pending::Ready {
                        request,
                        origin,
                        upload,
                    },
                ),
            ),
        }
    }

    fn begin(
        &self,
    ) -> Result<(HttpRequestBuilder, Instance<B, Request<B>>, Option<Upload<B>>), Error> {
        let mut state = self.state.borrow_mut();

        match std::mem::replace(&mut *state, Pending::Sending) {
            Pending::Ready {
                request,
                origin,
                upload,
            } => Ok((request, origin, upload)),
            previous => {
                *state = previous;

                Err(
                    Raise::<B>::new(ExceptionClass::builtin("RuntimeError"))
                        .arg("PendingResponse can only be awaited once")
                        .into()
                )
            }
        }
    }

    fn finish(&self) -> Option<ResponseBodyHandle> {
        let mut state = self.state.borrow_mut();

        match std::mem::replace(&mut *state, Pending::Done) {
            Pending::Entered(body) => Some(body),
            previous => {
                *state = previous;

                None
            }
        }
    }

    async fn send(
        request: HttpRequestBuilder,
        upload: Option<Upload<B>>,
    ) -> Result<HttpResponse, SendError> {
        match upload {
            Some(upload) => {
                let (response, pumped) = futures::future::join(
                    request.send(),
                    upload.pump(),
                )
                .await;

                pumped.map_err(SendError::Guest)?;

                response.map_err(SendError::Host)
            }
            None => request.send().await.map_err(SendError::Host),
        }
    }
}

#[host_class(backend = B, crate_path = agentc_executor_python::guestpy)]
impl<B: ExecutorBackend> PendingResponse<B> {
    #[guestpy(async_method, dunder = "__await__")]
    fn wait(
        &self,
    ) -> Result<impl Future<Output = Result<Response<B>, Error>> + use<B>, Error> {
        let (request, origin, upload) = self.begin()?;
        let state = self.state.clone();
        let exchange = Exchange::Sending {
            request: origin.clone(),
        };

        Ok(
            async move {
                match Self::send(request, upload).await {
                    Ok(response) => {
                        *state.borrow_mut() = Pending::Done;

                        Ok(
                            Response::from_response(response, origin),
                        )
                    }
                    Err(SendError::Guest(error)) => {
                        *state.borrow_mut() = Pending::Done;

                        Err(error)
                    }
                    Err(SendError::Host(error)) => {
                        *state.borrow_mut() = Pending::Done;

                        Err(
                            exchange.raise(error).into(),
                        )
                    }
                }
            },
        )
    }

    #[guestpy(async_method, dunder = "__aenter__")]
    fn enter(
        &self,
    ) -> Result<impl Future<Output = Result<Response<B>, Error>> + use<B>, Error> {
        let (request, origin, upload) = self.begin()?;
        let state = self.state.clone();
        let exchange = Exchange::Sending {
            request: origin.clone(),
        };

        Ok(
            async move {
                match Self::send(request, upload).await {
                    Ok(response) => {
                        let response = Response::from_response(response, origin);

                        *state.borrow_mut() = Pending::Entered(response.body_handle());

                        Ok(response)
                    }
                    Err(SendError::Guest(error)) => {
                        *state.borrow_mut() = Pending::Done;

                        Err(error)
                    }
                    Err(SendError::Host(error)) => {
                        *state.borrow_mut() = Pending::Done;

                        Err(
                            exchange.raise(error).into(),
                        )
                    }
                }
            },
        )
    }

    #[guestpy(async_method, dunder = "__aexit__")]
    fn exit(
        &self,
        _exc_type: Object<B>,
        _exc_value: Object<B>,
        _traceback: Object<B>,
    ) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let body = self.finish();

        Ok(
            async move {
                if let Some(body) = body {
                    body.release();
                }

                Ok(())
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use agentc_executor_python::{
        executor::Executor,
        guestpy::{
            bundle::Bundle,
            handle::{Instance, ObjectProtocol},
            rustpython::RustPython,
        },
    };

    use super::*;
    use crate::client::{client::HttpClient, python::executor::ExecutorBuilderHttpExt};

    const SOURCE: &str = r#"
from agentc_http import Request


def make_request():
    return Request("GET", "https://example.test/")
"#;

    #[tokio::test]
    async fn a_pending_response_can_begin_once() {
        let executor = Executor::<RustPython>::builder("agentc_http_pending_test")
            .bundle(
                Bundle::single("agentc_http_pending_test", SOURCE).expect("bundle builds"),
            )
            .workers(1)
            .with_http(HttpClient::builder())
            .build()
            .await
            .expect("executor builds");

        executor
            .execute(
                |context| Box::pin(
                    async move {
                        let origin = context
                            .module()
                            .function("make_request")?
                            .call::<_, Instance<RustPython, Request<RustPython>>>(())?;
                        let pending = PendingResponse::new(
                            HttpClient::builder()
                                .build()
                                .expect("client builds")
                                .get("https://example.test/"),
                            origin,
                            None,
                        );

                        assert!(pending.begin().is_ok());
                        assert!(pending.begin().is_err());
                        assert!(pending.finish().is_none());
                        assert!(pending.begin().is_err());

                        Ok(())
                    },
                ),
            )
            .await
            .expect("guest operation succeeds");

        executor.shutdown().await.expect("executor shuts down");
    }
}
