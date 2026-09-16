// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

mod upload;

pub(crate) mod exchange;
pub(crate) mod timeout;

pub mod body;
pub mod client;
pub mod exceptions;
pub mod executor;
pub mod headers;
pub mod library;
pub mod module;
pub mod params;
pub mod pending;
pub mod request;
pub mod response;

pub use crate::client::python::{executor::ExecutorBuilderHttpExt, library::HttpLibrary};

#[cfg(test)]
mod tests {
    use std::{
        convert::Infallible,
        net::SocketAddr,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use agentc_executor_python::{
        executor::Executor,
        guestpy::{
            bundle::Bundle,
            handle::{Coroutine, ObjectProtocol},
            rustpython::RustPython,
        },
        json::Json,
    };
    use axum::{
        Router,
        body::Body,
        extract::{Path, State},
        response::Redirect,
        routing::{any, get, post},
    };
    use bytes::Bytes;
    use futures::stream;
    use http::{HeaderMap, Method, StatusCode, Uri};
    use serde_json::{Value, json};
    use tokio::net::TcpListener;

    use super::ExecutorBuilderHttpExt;
    use crate::client::client::HttpClient;

    const SOURCE: &str = r#"
import asyncio
import agentc_http


async def redirect_limit(url):
    try:
        await agentc_http.Client().get(url)
    except agentc_http.TooManyRedirects as error:
        return error.request.url == url

    return False


async def composed(base_url):
    client = agentc_http.Client(
        base_url=base_url,
        params={"a": "1", "b": ["1", "2"]},
        headers=[("x-default", "yes"), ("x-repeated", "old")],
        timeout=10.0,
    )
    response = await client.post(
        "/echo?from=url",
        params=[("b", "3"), ("c", "4")],
        headers=[("x-repeated", "new-1"), ("x-repeated", "new-2")],
        body=agentc_http.Json({"ok": True}),
    )
    return await response.body.json()


async def url_case(value):
    kind, base_url = value.split("|", 1)
    client = agentc_http.Client(base_url=base_url or None)

    try:
        response = await client.get(
            "https://other.example/echo" if kind == "absolute" else "/echo"
        )
    except agentc_http.InvalidRequest as error:
        return str(error)

    return await response.body.json()


async def url_table(value):
    base_url, url = value.split("|", 1)
    client = agentc_http.Client(base_url=base_url or None)

    try:
        response = await client.get(url)
    except agentc_http.InvalidRequest as error:
        return str(error)

    result = response.url
    await response.close()
    return result


def invalid_base(value):
    try:
        agentc_http.Client(base_url=value)
    except ValueError as error:
        return str(error)

    return "no error"


async def body_kinds(url):
    client = agentc_http.Client()
    result = []

    async def stream_body():
        yield b"strea"
        yield b"med"

    for body in (
        b"bytes",
        "text",
        agentc_http.Json({"ok": True}),
        agentc_http.Form([("a", "x"), ("a", "y")]),
        stream_body(),
    ):
        response = await client.post(url, body=body)
        result.append(await response.body.json())

    for body in (
        "text",
        agentc_http.Json({"ok": True}),
        agentc_http.Form({"a": "x"}),
    ):
        response = await client.post(
            url,
            body=body,
            headers={"Content-Type": "application/custom"},
        )
        result.append(await response.body.json())

    return result


async def methods(url):
    client = agentc_http.Client()
    result = []

    for method in (
        client.get,
        client.head,
        client.options,
        client.delete,
        client.post,
        client.put,
        client.patch,
    ):
        response = await method(url)
        result.append(response.request.method)
        await response.close()

    return result


async def send_identity(url):
    request = agentc_http.Request("GET", url)

    try:
        await agentc_http.Client().send(request)
    except agentc_http.RequestError as error:
        return error.request is request

    return False


async def response_values(url):
    response = await agentc_http.Client().get(url)
    values = [
        response.status,
        response.reason,
        response.is_success,
        response.url,
        response.headers["content-type"],
        response.raise_for_status() is response,
        response.request.method,
    ]
    await response.close()
    return values


async def status_identity(url):
    response = await agentc_http.Client().get(url)

    try:
        response.raise_for_status()
    except agentc_http.HTTPStatusError as error:
        result = error.response is response
        await response.close()
        return result

    return False


async def body_state(url):
    response = await agentc_http.Client().get(url)
    first = await response.body.read()

    try:
        await response.body.text()
    except agentc_http.BodyConsumed as error:
        return [first.decode(), str(error), response.body.consumed, response.body.closed]

    return ["no error"]


async def closed_state(url):
    response = await agentc_http.Client().get(url)
    await response.close()

    try:
        await response.body.read()
    except agentc_http.BodyClosed as error:
        return [str(error), response.body.consumed, response.body.closed]

    return ["no error"]


async def decoded_values(url):
    client = agentc_http.Client()
    raw = await client.get(url)
    chunks = []
    async for chunk in raw.body:
        chunks.append(chunk)

    text_response = await client.get(url)
    text = await text_response.body.text()
    chunk_response = await client.get(url)
    text_chunks = [part async for part in chunk_response.body.text_chunks()]
    line_response = await client.get(url)
    lines = [line async for line in line_response.body.lines()]
    return [b"".join(chunks).decode("utf-8"), text, text_chunks, lines]


async def latin1_text(url):
    response = await agentc_http.Client().get(url)
    return await response.body.text()


async def invalid_json(url):
    response = await agentc_http.Client().get(url)

    try:
        await response.body.json()
    except agentc_http.DecodingError as error:
        return error.response is response

    return False


async def limited(value):
    url, operation = value.split("|", 1)
    response = await agentc_http.Client().get(url)

    try:
        if operation == "read":
            await response.body.read()
        elif operation == "text":
            await response.body.text()
        elif operation == "json":
            await response.body.json()
        elif operation == "bytes":
            async for _ in response.body:
                pass
        elif operation == "text_chunks":
            async for _ in response.body.text_chunks():
                pass
        else:
            async for _ in response.body.lines():
                pass
    except agentc_http.ResponseTooLarge as error:
        return [error.limit, error.response is response]

    return ["no error"]


async def pending_once(url):
    pending = agentc_http.Client().get(url)
    async with pending as response:
        first = await response.body.text()

    try:
        await pending
    except RuntimeError as error:
        return [first, str(error)]

    return ["no error"]


async def unread_context(url):
    async with agentc_http.Client().get(url) as response:
        body = response.body

    return [body.consumed, body.closed]


async def upload(url):
    async def chunks():
        yield b"first "
        yield b"second"

    response = await agentc_http.Client().post(url, body=chunks())
    return await response.body.json()


async def upload_twice(url):
    async def chunks():
        yield b"payload"

    client = agentc_http.Client()
    request = agentc_http.Request("POST", url, body=chunks())
    await client.send(request)

    try:
        client.send(request)
    except agentc_http.BodyConsumed as error:
        return str(error)

    return "no error"


async def upload_failure(url):
    async def chunks():
        yield b"before"
        raise LookupError("upload failed")

    try:
        await agentc_http.Client().post(url, body=chunks())
    except LookupError as error:
        return str(error)

    return "no error"


async def blocked_upload(base_url):
    async def chunks():
        yield b"first"
        await asyncio.Event().wait()
        yield b"second"

    await agentc_http.Client().post(
        base_url + "/observe-upload",
        body=chunks(),
    )


async def streamed_redirect(url):
    async def chunks():
        yield b"payload"

    response = await agentc_http.Client().post(url, body=chunks())
    status = response.status
    await response.close()
    return status


async def timeout(url):
    try:
        await agentc_http.Client(timeout=0.01).get(url)
    except agentc_http.TimeoutException as error:
        return str(error)

    return "no error"


async def deadline_case(value):
    url, timeout = value.split("|", 1)
    client = agentc_http.Client(
        timeout=None if timeout == "none" else float(timeout)
    )

    try:
        response = await client.get(url)
    except agentc_http.TimeoutException:
        return "timeout"

    await response.close()
    return "ok"


async def transport(url):
    try:
        await agentc_http.Client().get(url)
    except agentc_http.TransportError as error:
        return error.request.url == url

    return False
"#;

    struct TestState {
        echo_count: AtomicUsize,
        upload_bytes: AtomicUsize,
        first_upload_chunk: tokio::sync::Notify,
    }

    impl TestState {
        fn new() -> Self {
            Self {
                echo_count: AtomicUsize::new(0),
                upload_bytes: AtomicUsize::new(0),
                first_upload_chunk: tokio::sync::Notify::new(),
            }
        }
    }

    async fn echo(
        State(state): State<Arc<TestState>>,
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        body: Bytes,
    ) -> axum::Json<Value> {
        state
            .echo_count
            .fetch_add(1, Ordering::SeqCst);

        axum::Json(json!({
            "method": method.as_str(),
            "query": uri.query().unwrap_or_default(),
            "headers": {
                "x-default": headers
                    .get("x-default")
                    .and_then(|value| value.to_str().ok()),
                "x-repeated": headers
                    .get_all("x-repeated")
                    .iter()
                    .filter_map(|value| value.to_str().ok())
                    .collect::<Vec<_>>(),
                "content-type": headers
                    .get("content-type")
                    .and_then(|value| value.to_str().ok()),
            },
            "body": String::from_utf8_lossy(&body),
        }))
    }

    async fn status(Path(status): Path<u16>) -> (StatusCode, axum::Json<Value>) {
        (
            StatusCode::from_u16(status).expect("test status is valid"),
            axum::Json(json!({"status": status})),
        )
    }

    async fn chunks() -> http::Response<Body> {
        http::Response::builder()
            .header("content-type", "text/plain; charset=utf-8")
            .body(Body::from_stream(stream::iter([
                Ok::<_, Infallible>(Bytes::from_static(b"a\r")),
                Ok(Bytes::from_static(b"\n\xe2\x82")),
                Ok(Bytes::from_static(b"\xac\nb\rc")),
            ])))
            .expect("chunk response builds")
    }

    async fn latin1() -> http::Response<Body> {
        http::Response::builder()
            .header("content-type", "text/plain; charset=iso-8859-1")
            .body(Body::from(Bytes::from_static(b"\xe9")))
            .expect("charset response builds")
    }

    async fn observe_upload(State(state): State<Arc<TestState>>, body: Body) -> StatusCode {
        let mut chunks = body.into_data_stream();

        while let Some(chunk) = futures::StreamExt::next(&mut chunks).await {
            match chunk {
                Ok(chunk) => {
                    state
                        .upload_bytes
                        .fetch_add(chunk.len(), Ordering::SeqCst);
                    state.first_upload_chunk.notify_one();
                }
                Err(_) => break,
            }
        }

        StatusCode::OK
    }

    async fn slow() -> &'static str {
        std::future::pending::<()>().await;

        unreachable!()
    }

    async fn delayed() -> &'static str {
        tokio::time::sleep(Duration::from_millis(25)).await;

        "ready"
    }

    fn app(state: Arc<TestState>) -> Router {
        Router::new()
            .route("/echo", any(echo))
            .route("/v1/echo", any(echo))
            .route("/status/{status}", get(status))
            .route("/large", get(|| async { "x".repeat(64) }))
            .route("/invalid-json", get(|| async { "not json" }))
            .route("/slow", get(slow))
            .route("/delayed", get(delayed))
            .route("/chunks", get(chunks))
            .route("/latin1", get(latin1))
            .route("/observe-upload", post(observe_upload))
            .route("/redirect", post(|| async { Redirect::temporary("/echo") }))
            .route("/redirect-loop", get(|| async { Redirect::temporary("/redirect-loop") }))
            .with_state(state)
    }

    async fn server() -> (SocketAddr, Arc<TestState>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener binds");
        let address = listener
            .local_addr()
            .expect("test address exists");
        let state = Arc::new(TestState::new());
        let server_state = state.clone();

        drop(tokio::spawn(async move { axum::serve(listener, app(server_state)).await }));

        (address, state)
    }

    async fn executor(builder: crate::client::builder::HttpClientBuilder) -> Executor<RustPython> {
        Executor::<RustPython>::builder("agentc_http_integration_test")
            .bundle(Bundle::single("agentc_http_integration_test", SOURCE).expect("bundle builds"))
            .workers(2)
            .with_http(builder)
            .build()
            .await
            .expect("executor builds")
    }

    async fn call(
        executor: &Executor<RustPython>,
        export: &'static str,
        argument: String,
    ) -> Value {
        executor
            .execute(move |context| {
                Box::pin(async move {
                    context
                        .module()
                        .function(export)?
                        .call::<_, Coroutine<RustPython, Json>>((argument,))?
                        .await
                })
            })
            .await
            .expect("guest call succeeds")
            .into_inner()
    }

    #[tokio::test]
    async fn requests_apply_defaults_and_preserve_original_objects() {
        let (address, _) = server().await;
        let executor = executor(HttpClient::builder()).await;
        let base = format!("http://{address}");

        assert_eq!(
            call(&executor, "composed", base.clone()).await,
            json!({
                "method": "POST",
                "query": "from=url&a=1&b=3&c=4",
                "headers": {
                    "x-default": "yes",
                    "x-repeated": ["new-1", "new-2"],
                    "content-type": "application/json",
                },
                "body": "{\"ok\":true}",
            }),
        );
        assert_eq!(
            call(&executor, "url_case", String::from("relative|")).await,
            json!("invalid request: a relative URL needs a base_url"),
        );
        assert_eq!(
            call(&executor, "url_case", format!("absolute|{base}")).await,
            json!("invalid request: an absolute URL cannot be used with base_url"),
        );
        assert_eq!(call(&executor, "send_identity", String::from("/relative")).await, json!(true),);
        assert_eq!(
            call(&executor, "methods", format!("{base}/echo")).await,
            json!(["GET", "HEAD", "OPTIONS", "DELETE", "POST", "PUT", "PATCH"]),
        );
        assert_eq!(
            call(&executor, "url_table", format!("|{base}/echo")).await,
            json!(format!("{base}/echo")),
        );
        assert_eq!(
            call(&executor, "url_table", String::from("|/echo")).await,
            json!("invalid request: a relative URL needs a base_url"),
        );
        assert_eq!(
            call(&executor, "url_table", format!("{base}/v1|{base}/echo")).await,
            json!("invalid request: an absolute URL cannot be used with base_url"),
        );

        for relative in ["/echo", "echo"] {
            for suffix in ["/v1", "/v1/"] {
                assert_eq!(
                    call(&executor, "url_table", format!("{base}{suffix}|{relative}"),).await,
                    json!(format!("{base}/v1/echo")),
                );
            }
        }

        assert_eq!(
            call(&executor, "url_table", format!("{base}/v1|echo?page=2")).await,
            json!(format!("{base}/v1/echo?page=2")),
        );
        assert_eq!(
            call(&executor, "body_kinds", format!("{base}/echo"))
                .await
                .as_array()
                .expect("body cases are an array")
                .iter()
                .map(|case| { json!([case["body"], case["headers"]["content-type"],]) },)
                .collect::<Vec<_>>(),
            vec![
                json!(["bytes", null]),
                json!(["text", "text/plain; charset=utf-8"]),
                json!(["{\"ok\":true}", "application/json"]),
                json!(["a=x&a=y", "application/x-www-form-urlencoded"]),
                json!(["streamed", null]),
                json!(["text", "application/custom"]),
                json!(["{\"ok\":true}", "application/custom"]),
                json!(["a=x", "application/custom"]),
            ],
        );

        for invalid in [
            "",
            "/relative",
            "https://example.test/v1?x=1",
            "https://example.test/v1#fragment",
        ] {
            assert_eq!(
                executor
                    .execute(move |context| Box::pin(async move {
                        context
                            .module()
                            .function("invalid_base")?
                            .call::<_, String>((invalid,))
                    },),)
                    .await
                    .expect("guest call succeeds"),
                "base_url must be an absolute URL without a query string or fragment",
            );
        }

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn response_values_and_body_lifecycle_cross_the_binding() {
        let (address, count) = server().await;
        let executor = executor(HttpClient::builder()).await;
        let base = format!("http://{address}");

        assert_eq!(
            call(&executor, "response_values", format!("{base}/status/200")).await,
            json!([
                200,
                "OK",
                true,
                format!("{base}/status/200"),
                "application/json",
                true,
                "GET",
            ]),
        );
        assert_eq!(
            call(&executor, "status_identity", format!("{base}/status/404")).await,
            json!(true),
        );
        assert_eq!(
            call(&executor, "closed_state", format!("{base}/large")).await,
            json!(["body closed before consuming", false, true]),
        );
        assert_eq!(
            call(&executor, "body_state", format!("{base}/large")).await,
            json!(["x".repeat(64), "body already been consumed", true, true]),
        );

        let decoded = call(&executor, "decoded_values", format!("{base}/chunks")).await;

        assert_eq!(decoded[0], json!("a\r\n€\nb\rc"));
        assert_eq!(decoded[1], json!("a\r\n€\nb\rc"));
        assert_eq!(decoded[3], json!(["a", "€", "b", "c"]));
        assert_eq!(
            decoded[2]
                .as_array()
                .expect("decoded text chunks are an array")
                .iter()
                .map(|value| value
                    .as_str()
                    .expect("text chunk is a string"))
                .collect::<String>(),
            "a\r\n€\nb\rc",
        );
        assert_eq!(call(&executor, "latin1_text", format!("{base}/latin1")).await, json!("é"),);
        assert_eq!(
            call(&executor, "invalid_json", format!("{base}/invalid-json")).await,
            json!(true),
        );

        let before = count.echo_count.load(Ordering::SeqCst);

        assert_eq!(
            call(&executor, "pending_once", format!("{base}/echo")).await,
            json!([
                json!({
                    "method": "GET",
                    "query": "",
                    "headers": {
                        "x-default": null,
                        "x-repeated": [],
                        "content-type": null,
                    },
                    "body": "",
                })
                .to_string(),
                "PendingResponse can only be awaited once",
            ]),
        );
        assert_eq!(count.echo_count.load(Ordering::SeqCst), before + 1);
        assert_eq!(
            call(&executor, "unread_context", format!("{base}/large")).await,
            json!([false, true]),
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn every_response_read_path_enforces_the_same_size_limit() {
        let (address, _) = server().await;
        let executor = executor(HttpClient::builder().max_response_bytes(8u64)).await;

        for operation in ["read", "text", "json", "bytes", "text_chunks", "lines"] {
            assert_eq!(
                call(&executor, "limited", format!("http://{address}/large|{operation}"),).await,
                json!([8, true]),
            );
        }

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn guest_timeout_cannot_extend_the_host_deadline() {
        let (address, _) = server().await;
        let url = format!("http://{address}/delayed");
        let host_only =
            executor(HttpClient::builder().request_timeout(Duration::from_millis(10))).await;

        assert_eq!(
            call(&host_only, "deadline_case", format!("{url}|none")).await,
            json!("timeout"),
        );
        assert_eq!(call(&host_only, "deadline_case", format!("{url}|0.1")).await, json!("timeout"),);

        host_only
            .shutdown()
            .await
            .expect("executor shuts down");

        let guest_only = executor(HttpClient::builder()).await;

        assert_eq!(
            call(&guest_only, "deadline_case", format!("{url}|0.01")).await,
            json!("timeout"),
        );
        assert_eq!(call(&guest_only, "deadline_case", format!("{url}|none")).await, json!("ok"),);

        guest_only
            .shutdown()
            .await
            .expect("executor shuts down");

        let shorter_guest =
            executor(HttpClient::builder().request_timeout(Duration::from_millis(100))).await;

        assert_eq!(
            call(&shorter_guest, "deadline_case", format!("{url}|0.01")).await,
            json!("timeout"),
        );

        shorter_guest
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn streamed_upload_and_transport_errors_reach_python() {
        let (address, _) = server().await;
        let executor = executor(HttpClient::builder()).await;
        let base = format!("http://{address}");

        assert_eq!(
            call(&executor, "upload", format!("{base}/echo")).await["body"],
            json!("first second"),
        );
        assert_eq!(
            call(&executor, "upload_twice", format!("{base}/echo")).await,
            json!("body already been consumed"),
        );
        assert_eq!(
            call(&executor, "upload_failure", format!("{base}/echo")).await,
            json!("upload failed"),
        );
        assert_eq!(
            call(&executor, "streamed_redirect", format!("{base}/redirect")).await,
            json!(307),
        );
        assert_eq!(
            call(&executor, "redirect_limit", format!("{base}/redirect-loop")).await,
            json!(true),
        );
        assert_eq!(
            call(&executor, "timeout", format!("{base}/slow")).await,
            json!("request timed out"),
        );

        let unused = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("unused test listener binds");
        let unavailable = unused
            .local_addr()
            .expect("unused address exists");

        drop(unused);

        assert_eq!(
            call(&executor, "transport", format!("http://{unavailable}/")).await,
            json!(true),
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn executor_cancellation_stops_a_blocked_upload() {
        let (address, state) = server().await;
        let cancellation = tokio_util::sync::CancellationToken::new();
        let executor = Executor::<RustPython>::builder("agentc_http_integration_test")
            .bundle(Bundle::single("agentc_http_integration_test", SOURCE).expect("bundle builds"))
            .workers(1)
            .cancellation(cancellation.clone())
            .with_http(HttpClient::builder())
            .build()
            .await
            .expect("executor builds");
        let active = executor.clone();
        let base = format!("http://{address}");
        let in_flight = tokio::spawn(async move {
            active
                .execute(move |context| {
                    Box::pin(async move {
                        context
                            .module()
                            .function("blocked_upload")?
                            .call::<_, Coroutine<RustPython, ()>>((base,))?
                            .await
                    })
                })
                .await
        });

        if state
            .upload_bytes
            .load(Ordering::SeqCst)
            == 0
        {
            state
                .first_upload_chunk
                .notified()
                .await;
        }

        cancellation.cancel();

        assert!(
            in_flight
                .await
                .expect("caller joins")
                .is_err(),
        );
        assert_eq!(
            state
                .upload_bytes
                .load(Ordering::SeqCst),
            5
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }
}
