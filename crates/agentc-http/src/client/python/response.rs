// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    cell::RefCell,
    collections::VecDeque,
    future::{Future, poll_fn},
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{
        errors::Error,
        handle::{Instance, Object},
        host::{exception::Raise, iter::HostStream},
        host_class,
    },
};
use bytes::{Bytes, BytesMut};
use encoding_rs::{CoderResult, Decoder, Encoding, UTF_8};
use futures::{Stream, stream::try_unfold};
use http::{HeaderMap, StatusCode, header::CONTENT_TYPE};
use url::Url;

use crate::client::{
    errors::HttpClientError,
    python::{
        exceptions::{BodyClosed, BodyConsumed, HttpStatusError},
        exchange::Exchange,
        headers::Headers,
        request::Request,
    },
    response::{HttpBodyStream, HttpResponse},
};

enum ResponseBodyState {
    Unread(HttpBodyStream),
    Reading(HttpBodyStream),
    Released { consumed: bool },
}

#[derive(Clone)]
pub(crate) struct ResponseBodyHandle {
    state: Rc<RefCell<ResponseBodyState>>,
}

impl ResponseBodyHandle {
    fn new(body: HttpBodyStream) -> Self {
        Self {
            state: Rc::new(RefCell::new(ResponseBodyState::Unread(body))),
        }
    }

    fn begin<B: ExecutorBackend>(&self) -> Result<(), Error> {
        let mut state = self.state.borrow_mut();

        match std::mem::replace(&mut *state, ResponseBodyState::Released { consumed: false }) {
            ResponseBodyState::Unread(body) => {
                *state = ResponseBodyState::Reading(body);

                Ok(())
            }
            ResponseBodyState::Reading(body) => {
                *state = ResponseBodyState::Reading(body);

                Err(Raise::<B>::host(BodyConsumed {
                    message: String::from("body already been consumed"),
                })
                .into())
            }
            ResponseBodyState::Released { consumed } => {
                *state = ResponseBodyState::Released { consumed };

                Err(if consumed {
                    Raise::<B>::host(BodyConsumed {
                        message: String::from("body already been consumed"),
                    })
                    .into()
                } else {
                    Raise::<B>::host(BodyClosed {
                        message: String::from("body closed before consuming"),
                    })
                    .into()
                })
            }
        }
    }

    fn poll_chunk<B: ExecutorBackend>(
        &self,
        context: &mut Context<'_>,
        exchange: &Exchange<B>,
    ) -> Poll<Option<Result<Bytes, Error>>> {
        let mut state = self.state.borrow_mut();

        match &mut *state {
            ResponseBodyState::Reading(body) => match Pin::new(body).poll_next(context) {
                Poll::Ready(Some(Ok(chunk))) => Poll::Ready(Some(Ok(chunk))),
                Poll::Ready(Some(Err(error))) => {
                    *state = ResponseBodyState::Released { consumed: true };

                    Poll::Ready(Some(Err(exchange.raise(error).into())))
                }
                Poll::Ready(None) => {
                    *state = ResponseBodyState::Released { consumed: true };

                    Poll::Ready(None)
                }
                Poll::Pending => Poll::Pending,
            },
            ResponseBodyState::Unread(_) => {
                unreachable!("response body must begin before it is polled")
            }
            ResponseBodyState::Released { .. } => {
                Poll::Ready(Some(Err(Raise::<B>::host(BodyClosed {
                    message: String::from("body closed before consuming"),
                })
                .into())))
            }
        }
    }

    fn consumed(&self) -> bool {
        matches!(
            &*self.state.borrow(),
            ResponseBodyState::Reading(_) | ResponseBodyState::Released { consumed: true }
        )
    }

    fn closed(&self) -> bool {
        matches!(&*self.state.borrow(), ResponseBodyState::Released { .. })
    }

    pub(crate) fn release(&self) {
        let mut state = self.state.borrow_mut();
        let consumed = matches!(
            &*state,
            ResponseBodyState::Reading(_) | ResponseBodyState::Released { consumed: true }
        );

        *state = ResponseBodyState::Released { consumed };
    }
}

struct BodyReader<B: ExecutorBackend> {
    handle: ResponseBodyHandle,
    exchange: Exchange<B>,
}

impl<B: ExecutorBackend> BodyReader<B> {
    fn new(handle: ResponseBodyHandle, exchange: Exchange<B>) -> Result<Self, Error> {
        handle.begin::<B>()?;

        Ok(Self { handle, exchange })
    }

    async fn next(&self) -> Result<Option<Bytes>, Error> {
        poll_fn(|context| {
            self.handle
                .poll_chunk(context, &self.exchange)
        })
        .await
        .transpose()
    }

    async fn read(self) -> Result<Bytes, Error> {
        let mut collected = BytesMut::new();

        while let Some(chunk) = self.next().await? {
            collected.extend_from_slice(&chunk);
        }

        Ok(collected.freeze())
    }

    fn into_stream(self) -> impl Stream<Item = Result<Bytes, Error>> {
        try_unfold(self, |reader| async move {
            Ok(reader
                .next()
                .await?
                .map(|chunk| (chunk, reader)))
        })
    }
}

struct TextDecoder {
    inner: Decoder,
}

impl TextDecoder {
    fn new(headers: &HeaderMap) -> Self {
        Self {
            inner: headers
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| {
                    value
                        .split(';')
                        .skip(1)
                        .filter_map(|parameter| parameter.split_once('='))
                        .find_map(|(name, value)| {
                            name.trim()
                                .eq_ignore_ascii_case("charset")
                                .then(|| value.trim().trim_matches(['\'', '"']))
                        })
                })
                .and_then(|label| Encoding::for_label(label.as_bytes()))
                .unwrap_or(UTF_8)
                .new_decoder_with_bom_removal(),
        }
    }

    fn decode(&mut self, bytes: &[u8], last: bool) -> String {
        let mut decoded = String::new();
        let mut read = 0;

        loop {
            let (result, consumed, _) =
                self.inner
                    .decode_to_string(&bytes[read..], &mut decoded, last);
            read += consumed;

            match result {
                CoderResult::InputEmpty => return decoded,
                CoderResult::OutputFull => decoded.reserve(
                    bytes
                        .len()
                        .saturating_sub(read)
                        .saturating_mul(3)
                        .max(32),
                ),
            }
        }
    }
}

struct TextReader<B: ExecutorBackend> {
    body: BodyReader<B>,
    decoder: TextDecoder,
    finished: bool,
}

impl<B: ExecutorBackend> TextReader<B> {
    fn new(
        handle: ResponseBodyHandle,
        exchange: Exchange<B>,
        headers: &HeaderMap,
    ) -> Result<Self, Error> {
        Ok(Self {
            body: BodyReader::new(handle, exchange)?,
            decoder: TextDecoder::new(headers),
            finished: false,
        })
    }

    async fn next(&mut self) -> Result<Option<String>, Error> {
        loop {
            if self.finished {
                return Ok(None);
            }

            match self.body.next().await? {
                Some(chunk) => {
                    let decoded = self.decoder.decode(&chunk, false);

                    if !decoded.is_empty() {
                        return Ok(Some(decoded));
                    }
                }
                None => {
                    self.finished = true;

                    return Ok(match self.decoder.decode(&[], true) {
                        decoded if decoded.is_empty() => None,
                        decoded => Some(decoded),
                    });
                }
            }
        }
    }

    async fn read(mut self) -> Result<String, Error> {
        let mut collected = String::new();

        while let Some(chunk) = self.next().await? {
            collected.push_str(&chunk);
        }

        Ok(collected)
    }

    fn into_stream(self) -> impl Stream<Item = Result<String, Error>> {
        try_unfold(self, |mut reader| async move {
            Ok(reader
                .next()
                .await?
                .map(|chunk| (chunk, reader)))
        })
    }
}

struct LineReader<B: ExecutorBackend> {
    text: TextReader<B>,
    buffer: String,
    ready: VecDeque<String>,
    finished: bool,
}

impl<B: ExecutorBackend> LineReader<B> {
    fn new(text: TextReader<B>) -> Self {
        Self {
            text,
            buffer: String::new(),
            ready: VecDeque::new(),
            finished: false,
        }
    }

    fn split_lines(&mut self, final_chunk: bool) {
        let bytes = self.buffer.as_bytes();
        let mut line = 0;
        let mut cursor = 0;

        while cursor < bytes.len() {
            let terminator = match bytes[cursor] {
                b'\n' => Some(1),
                b'\r' if cursor + 1 < bytes.len() && bytes[cursor + 1] == b'\n' => Some(2),
                b'\r' if cursor + 1 < bytes.len() || final_chunk => Some(1),
                _ => None,
            };

            match terminator {
                Some(length) => {
                    self.ready
                        .push_back(self.buffer[line..cursor].to_owned());
                    cursor += length;
                    line = cursor;
                }
                None => cursor += 1,
            }
        }

        if final_chunk && line < self.buffer.len() {
            self.ready
                .push_back(self.buffer[line..].to_owned());
            line = self.buffer.len();
        }

        self.buffer.drain(..line);
    }

    async fn next(&mut self) -> Result<Option<String>, Error> {
        loop {
            if let Some(line) = self.ready.pop_front() {
                return Ok(Some(line));
            }

            if self.finished {
                return Ok(None);
            }

            match self.text.next().await? {
                Some(decoded) => {
                    self.buffer.push_str(&decoded);
                    self.split_lines(false);
                }
                None => {
                    self.finished = true;
                    self.split_lines(true);
                }
            }
        }
    }

    fn into_stream(self) -> impl Stream<Item = Result<String, Error>> {
        try_unfold(self, |mut reader| async move {
            Ok(reader
                .next()
                .await?
                .map(|line| (line, reader)))
        })
    }
}

pub struct ResponseBody<B: ExecutorBackend> {
    handle: ResponseBodyHandle,
    exchange: Exchange<B>,
}

impl<B: ExecutorBackend> ResponseBody<B> {
    fn body_reader(&self) -> Result<BodyReader<B>, Error> {
        BodyReader::new(self.handle.clone(), self.exchange.clone())
    }

    fn text_reader(&self) -> Result<TextReader<B>, Error> {
        match &self.exchange {
            Exchange::Streaming { response, .. } => response.borrow_with(|response| {
                TextReader::new(self.handle.clone(), self.exchange.clone(), &response.headers)
            })?,
            Exchange::Sending { .. } => {
                unreachable!("a response body always belongs to a streaming exchange")
            }
        }
    }
}

#[host_class(backend = B, crate_path = agentc_executor_python::guestpy)]
impl<B: ExecutorBackend> ResponseBody<B> {
    #[guestpy(get)]
    fn consumed(&self) -> Result<bool, Error> {
        Ok(self.handle.consumed())
    }

    #[guestpy(get)]
    fn closed(&self) -> Result<bool, Error> {
        Ok(self.handle.closed())
    }

    #[guestpy(dunder = "__aiter__")]
    fn chunks(&self) -> Result<HostStream<Bytes>, Error> {
        Ok(HostStream::new(self.body_reader()?.into_stream()))
    }

    #[guestpy(method)]
    fn text_chunks(&self) -> Result<HostStream<String>, Error> {
        Ok(HostStream::new(self.text_reader()?.into_stream()))
    }

    #[guestpy(method)]
    fn lines(&self) -> Result<HostStream<String>, Error> {
        Ok(HostStream::new(LineReader::new(self.text_reader()?).into_stream()))
    }

    #[guestpy(async_method)]
    fn read(&self) -> Result<impl Future<Output = Result<Bytes, Error>> + use<B>, Error> {
        Ok(self.body_reader()?.read())
    }

    #[guestpy(async_method)]
    fn text(&self) -> Result<impl Future<Output = Result<String, Error>> + use<B>, Error> {
        Ok(self.text_reader()?.read())
    }

    #[guestpy(async_method)]
    fn json(
        &self,
    ) -> Result<
        impl Future<Output = Result<agentc_executor_python::json::Json, Error>> + use<B>,
        Error,
    > {
        let reader = self.body_reader()?;
        let exchange = self.exchange.clone();

        Ok(async move {
            serde_json::from_slice(&reader.read().await?)
                .map(agentc_executor_python::json::Json)
                .map_err(|error| {
                    exchange
                        .raise(HttpClientError::decode(error.to_string()))
                        .into()
                })
        })
    }
}

pub struct Response<B: ExecutorBackend> {
    status: StatusCode,
    url: Url,
    headers: HeaderMap,
    request: Instance<B, Request<B>>,
    body: ResponseBodyHandle,
}

impl<B: ExecutorBackend> Response<B> {
    pub(crate) fn from_response(response: HttpResponse, request: Instance<B, Request<B>>) -> Self {
        Self {
            status: response.status(),
            url: response.url().clone(),
            headers: response.headers().clone(),
            request,
            body: ResponseBodyHandle::new(response.into_stream()),
        }
    }

    fn status_kind(&self) -> &'static str {
        if self.status.is_informational() {
            "Informational"
        } else if self.status.is_redirection() {
            "Redirect"
        } else if self.status.is_client_error() {
            "Client error"
        } else if self.status.is_server_error() {
            "Server error"
        } else {
            "Invalid status code"
        }
    }

    fn status_message(&self) -> String {
        match self.status.canonical_reason() {
            Some(reason) => format!("{} {reason}", self.status.as_u16(),),
            None => self.status.as_u16().to_string(),
        }
    }

    pub(crate) fn body_handle(&self) -> ResponseBodyHandle {
        self.body.clone()
    }
}

#[host_class(backend = B, crate_path = agentc_executor_python::guestpy)]
impl<B: ExecutorBackend> Response<B> {
    #[guestpy(get)]
    fn status(&self) -> Result<u16, Error> {
        Ok(self.status.as_u16())
    }

    #[guestpy(get)]
    fn reason(&self) -> Result<String, Error> {
        Ok(self
            .status
            .canonical_reason()
            .unwrap_or_default()
            .to_owned())
    }

    #[guestpy(get)]
    fn is_success(&self) -> Result<bool, Error> {
        Ok(self.status.is_success())
    }

    #[guestpy(get)]
    fn url(&self) -> Result<String, Error> {
        Ok(self.url.to_string())
    }

    #[guestpy(get)]
    fn headers(&self) -> Result<Headers<B>, Error> {
        Ok(Headers::from_map(self.headers.clone()))
    }

    #[guestpy(get)]
    fn request(&self) -> Result<Instance<B, Request<B>>, Error> {
        Ok(self.request.clone())
    }

    #[guestpy(get)]
    fn body(
        &self,
        #[guestpy(this)] this: Instance<B, Response<B>>,
    ) -> Result<ResponseBody<B>, Error> {
        Ok(ResponseBody {
            handle: self.body.clone(),
            exchange: Exchange::Streaming {
                request: self.request.clone(),
                response: this,
            },
        })
    }

    #[guestpy(method)]
    fn raise_for_status(
        &self,
        #[guestpy(this)] this: Instance<B, Response<B>>,
    ) -> Result<Instance<B, Response<B>>, Error> {
        if self.status.is_success() {
            return Ok(this);
        }

        Err(Raise::host(HttpStatusError {
            message: format!(
                "{} '{}' for url '{}'",
                self.status_kind(),
                self.status_message(),
                self.url,
            ),
            response: this,
        })
        .into())
    }

    #[guestpy(async_method)]
    fn close(&self) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let body = self.body.clone();

        Ok(async move {
            body.release();

            Ok(())
        })
    }

    #[guestpy(async_method, dunder = "__aenter__")]
    fn enter(
        &self,
        #[guestpy(this)] this: Instance<B, Response<B>>,
    ) -> Result<impl Future<Output = Result<Instance<B, Response<B>>, Error>> + use<B>, Error> {
        Ok(async move { Ok(this) })
    }

    #[guestpy(async_method, dunder = "__aexit__")]
    fn exit(
        &self,
        _exc_type: Object<B>,
        _exc_value: Object<B>,
        _traceback: Object<B>,
    ) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let body = self.body.clone();

        Ok(async move {
            body.release();

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use agentc_executor_python::guestpy::rustpython::RustPython;
    use http::HeaderValue;

    use super::*;

    #[test]
    fn decodes_the_response_charset_incrementally() {
        let mut headers = HeaderMap::new();

        let mut decoder = TextDecoder::new(&headers);

        assert_eq!(decoder.decode(&[0xe2, 0x82], false), "");
        assert_eq!(decoder.decode(&[0xac], false), "€");
        assert_eq!(decoder.decode(&[], true), "");

        headers
            .insert(CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=\"iso-8859-1\""));

        let mut decoder = TextDecoder::new(&headers);

        assert_eq!(decoder.decode(&[0xe9], true), "é");

        headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=unknown"));

        let mut decoder = TextDecoder::new(&headers);

        assert_eq!(decoder.decode(&[0xff], true), "�");
    }

    #[test]
    fn response_body_can_be_claimed_once_and_then_released() {
        let response = HttpResponse::new(
            reqwest::Response::from(
                http::Response::builder()
                    .status(200)
                    .body(reqwest::Body::from("payload"))
                    .expect("test response builds"),
            ),
            None,
            None,
        );
        let handle = ResponseBodyHandle::new(response.into_stream());

        assert!(!handle.consumed());
        assert!(!handle.closed());
        assert!(handle.begin::<RustPython>().is_ok());
        assert!(handle.consumed());
        assert!(!handle.closed());
        assert!(handle.begin::<RustPython>().is_err());

        handle.release();

        assert!(handle.consumed());
        assert!(handle.closed());
    }
}
