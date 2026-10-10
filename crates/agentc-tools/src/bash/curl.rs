// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::time::Duration;

use agentc_http::{
    client::{HttpClient, errors::HttpClientError},
    protocol::{
        HeaderMap, HeaderName, HeaderValue, Method, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE, COOKIE, REFERER, USER_AGENT},
    },
};
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use bashkit::{Builtin, BuiltinContext, ExecResult};
use uuid::Uuid;

const CURL_HELP: &str = "Usage: curl [OPTIONS] URL\n";
const DEFAULT_USER_AGENT: &str = "curl/8.7.1";
const MAX_REQUEST_BODY_BYTES: usize = 10 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq)]
enum DataKind {
    Data,
    Raw,
    Binary,
    UrlEncode,
}

struct DataPart {
    kind: DataKind,
    value: String,
}

impl DataPart {
    fn encode(bytes: &[u8]) -> Vec<u8> {
        let mut encoded = Vec::new();

        for byte in bytes {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                    encoded.push(*byte);
                }
                b' ' => encoded.push(b'+'),
                _ => encoded.extend_from_slice(format!("%{byte:02X}").as_bytes()),
            }
        }

        encoded
    }

    async fn resolve(
        &self,
        context: &BuiltinContext<'_>,
        stdin_available: &mut bool,
    ) -> Result<Vec<u8>, ExecResult> {
        if self.kind == DataKind::UrlEncode
            && !self.value.starts_with('@')
            && let Some((name, content)) = self.value.split_once('=')
        {
            let mut encoded = name.as_bytes().to_vec();

            encoded.push(b'=');
            encoded.extend_from_slice(&DataPart::encode(content.as_bytes()));

            return Ok(encoded);
        }

        let bytes = if self.kind != DataKind::Raw && self.value == "@-" {
            if *stdin_available {
                *stdin_available = false;
                context
                    .stdin
                    .map(|stdin| stdin.as_bytes().to_vec())
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        } else if self.kind != DataKind::Raw
            && let Some(path) = self.value.strip_prefix('@')
        {
            context
                .fs
                .read_file(&context.cwd.join(path))
                .await
                .map_err(|error| ExecResult::err(format!("curl: {error}\n"), 26))?
        } else {
            self.value.as_bytes().to_vec()
        };

        Ok(match self.kind {
            DataKind::Data => bytes
                .into_iter()
                .filter(|byte| !matches!(byte, b'\r' | b'\n'))
                .collect(),
            DataKind::UrlEncode => DataPart::encode(&bytes),
            DataKind::Raw | DataKind::Binary => bytes,
        })
    }
}

struct CurlOptions {
    url: Option<String>,
    method: Method,
    explicit_method: bool,
    data: Vec<DataPart>,
    headers: HeaderMap,
    forms: Vec<String>,
    output: Option<String>,
    remote_name: bool,
    get: bool,
    head: bool,
    silent: bool,
    show_error: bool,
    verbose: bool,
    fail: bool,
    write_out: Option<String>,
    timeout: Option<Duration>,
    basic_auth: Option<String>,
    user_agent: Option<String>,
    referer: Option<String>,
    cookie: Option<String>,
}

impl CurlOptions {
    fn new() -> Self {
        CurlOptions {
            url: None,
            method: Method::GET,
            explicit_method: false,
            data: Vec::new(),
            headers: HeaderMap::new(),
            forms: Vec::new(),
            output: None,
            remote_name: false,
            get: false,
            head: false,
            silent: false,
            show_error: false,
            verbose: false,
            fail: false,
            write_out: None,
            timeout: None,
            basic_auth: None,
            user_agent: None,
            referer: None,
            cookie: None,
        }
    }

    fn next_value(args: &[String], index: &mut usize) -> Result<String, ExecResult> {
        *index += 1;

        args.get(*index)
            .cloned()
            .ok_or_else(|| ExecResult::err("curl: option requires a value\n", 2))
    }

    fn parse(args: &[String]) -> Result<Self, ExecResult> {
        let mut options = CurlOptions::new();
        let mut index = 0;

        while let Some(argument) = args.get(index) {
            match argument.as_str() {
                "--help" => return Err(ExecResult::ok(CURL_HELP)),
                "--version" => return Err(ExecResult::ok("curl 8.7.1 (agentc)\n")),
                "-s" | "--silent" => options.silent = true,
                "-S" | "--show-error" => options.show_error = true,
                "-v" | "--verbose" => options.verbose = true,
                "-f" | "--fail" => options.fail = true,
                "-L" | "--location" | "--compressed" | "-k" | "--insecure" => {}
                "-I" | "--head" => {
                    options.head = true;
                    options.method = Method::HEAD;
                    options.explicit_method = true;
                }
                "-G" | "--get" => options.get = true,
                "-O" | "--remote-name" => options.remote_name = true,
                "-o" | "--output" => {
                    options.output = Some(Self::next_value(args, &mut index)?);
                }
                "-X" | "--request" => {
                    options.method = Self::next_value(args, &mut index)?
                        .parse()
                        .map_err(|_| ExecResult::err("curl: invalid request method\n", 2))?;
                    options.explicit_method = true;
                }
                "-d" | "--data" => options.data.push(DataPart {
                    kind: DataKind::Data,
                    value: Self::next_value(args, &mut index)?,
                }),
                "--data-raw" => options.data.push(DataPart {
                    kind: DataKind::Raw,
                    value: Self::next_value(args, &mut index)?,
                }),
                "--data-binary" => options.data.push(DataPart {
                    kind: DataKind::Binary,
                    value: Self::next_value(args, &mut index)?,
                }),
                "--data-urlencode" => options.data.push(DataPart {
                    kind: DataKind::UrlEncode,
                    value: Self::next_value(args, &mut index)?,
                }),
                "-H" | "--header" => {
                    let header = Self::next_value(args, &mut index)?;
                    let (name, value) = header
                        .split_once(':')
                        .ok_or_else(|| ExecResult::err("curl: invalid header\n", 2))?;

                    options.headers.insert(
                        HeaderName::from_bytes(name.as_bytes())
                            .map_err(|_| ExecResult::err("curl: invalid header name\n", 2))?,
                        HeaderValue::from_str(value.trim_start())
                            .map_err(|_| ExecResult::err("curl: invalid header value\n", 2))?,
                    );
                }
                "-w" | "--write-out" => {
                    options.write_out = Some(Self::next_value(args, &mut index)?);
                }
                "-m" | "--max-time" => {
                    options.timeout = Some(Duration::from_secs_f64(
                        Self::next_value(args, &mut index)?
                            .parse()
                            .map_err(|_| ExecResult::err("curl: invalid timeout\n", 2))?,
                    ));
                }
                "--connect-timeout" => {
                    Self::next_value(args, &mut index)?;
                }
                "-F" | "--form" => {
                    options
                        .forms
                        .push(Self::next_value(args, &mut index)?);
                }
                "-u" | "--user" => {
                    options.basic_auth = Some(Self::next_value(args, &mut index)?);
                }
                "-A" | "--user-agent" => {
                    options.user_agent = Some(Self::next_value(args, &mut index)?);
                }
                "-e" | "--referer" => {
                    options.referer = Some(Self::next_value(args, &mut index)?);
                }
                "-b" | "--cookie" => {
                    options.cookie = Some(Self::next_value(args, &mut index)?);
                }
                _ if argument.starts_with("-d") && argument.len() > 2 => {
                    options.data.push(DataPart {
                        kind: DataKind::Data,
                        value: argument[2..].to_owned(),
                    });
                }
                _ if argument.starts_with("--data=") => options.data.push(DataPart {
                    kind: DataKind::Data,
                    value: argument[7..].to_owned(),
                }),
                _ if argument.starts_with("--data-raw=") => options.data.push(DataPart {
                    kind: DataKind::Raw,
                    value: argument[11..].to_owned(),
                }),
                _ if argument.starts_with("--data-binary=") => options.data.push(DataPart {
                    kind: DataKind::Binary,
                    value: argument[14..].to_owned(),
                }),
                _ if argument.starts_with("--data-urlencode=") => options.data.push(DataPart {
                    kind: DataKind::UrlEncode,
                    value: argument[17..].to_owned(),
                }),
                _ if !argument.starts_with('-') => options.url = Some(argument.clone()),
                _ => {}
            }

            index += 1;
        }

        if !options.explicit_method
            && !options.get
            && (!options.data.is_empty() || !options.forms.is_empty())
        {
            options.method = Method::POST;
        }

        options
            .headers
            .entry(USER_AGENT)
            .or_insert(
                HeaderValue::from_str(
                    options
                        .user_agent
                        .as_deref()
                        .unwrap_or(DEFAULT_USER_AGENT),
                )
                .map_err(|_| ExecResult::err("curl: invalid user agent\n", 2))?,
            );

        if let Some(credentials) = &options.basic_auth {
            options.headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Basic {}", BASE64.encode(credentials)))
                    .map_err(|_| ExecResult::err("curl: invalid credentials\n", 2))?,
            );
        }

        if let Some(referer) = &options.referer {
            options.headers.insert(
                REFERER,
                HeaderValue::from_str(referer)
                    .map_err(|_| ExecResult::err("curl: invalid referer\n", 2))?,
            );
        }

        if let Some(cookie) = &options.cookie {
            options.headers.insert(
                COOKIE,
                HeaderValue::from_str(cookie)
                    .map_err(|_| ExecResult::err("curl: invalid cookie\n", 2))?,
            );
        }

        Ok(options)
    }

    fn error(&self, error: HttpClientError) -> ExecResult {
        let exit_code = match error {
            HttpClientError::Timeout => 28,
            HttpClientError::BodyTooLarge { .. } => 63,
            HttpClientError::TooManyRedirects => 47,
            HttpClientError::Denied { .. } | HttpClientError::Transport { .. } => 7,
            HttpClientError::InvalidRequest { .. } => 3,
            HttpClientError::Configuration { .. } | HttpClientError::Decode { .. } => 1,
        };

        ExecResult::err(
            if self.silent && !self.show_error {
                String::new()
            } else {
                format!("curl: {error}\n")
            },
            exit_code,
        )
    }

    async fn body(&mut self, context: &BuiltinContext<'_>) -> Result<Option<Vec<u8>>, ExecResult> {
        if !self.forms.is_empty() {
            return self.multipart(context).await.map(Some);
        }

        if self.data.is_empty() {
            return Ok(None);
        }

        let mut body = Vec::new();
        let mut stdin_available = true;

        for (index, part) in self.data.iter().enumerate() {
            if index > 0 {
                body.push(b'&');
            }

            body.extend_from_slice(
                &part
                    .resolve(context, &mut stdin_available)
                    .await?,
            );

            if body.len() > MAX_REQUEST_BODY_BYTES {
                return Err(ExecResult::err("curl: request body too large\n", 2));
            }
        }

        Ok(Some(body))
    }

    async fn multipart(&mut self, context: &BuiltinContext<'_>) -> Result<Vec<u8>, ExecResult> {
        let boundary = format!("agentc-{}", Uuid::new_v4().simple());
        let mut body = Vec::new();

        self.headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_str(&format!("multipart/form-data; boundary={boundary}"))
                .map_err(|_| ExecResult::err("curl: invalid multipart boundary\n", 2))?,
        );

        for form in &self.forms {
            let (name, value) = form
                .split_once('=')
                .ok_or_else(|| ExecResult::err("curl: invalid form field\n", 2))?;

            if [name, value]
                .iter()
                .any(|value| value.contains('\r') || value.contains('\n') || value.contains('"'))
            {
                return Err(ExecResult::err("curl: invalid multipart value\n", 2));
            }

            body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());

            if let Some(specification) = value.strip_prefix('@') {
                let (path, content_type) = specification
                    .split_once(";type=")
                    .map_or((specification, "application/octet-stream"), |parts| parts);
                let filename = context
                    .cwd
                    .join(path)
                    .file_name()
                    .and_then(|filename| filename.to_str())
                    .ok_or_else(|| ExecResult::err("curl: invalid upload filename\n", 26))?
                    .to_owned();

                body.extend_from_slice(
                    format!(
                        "Content-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n"
                    )
                    .as_bytes(),
                );
                body.extend_from_slice(
                    &context
                        .fs
                        .read_file(&context.cwd.join(path))
                        .await
                        .map_err(|error| ExecResult::err(format!("curl: {error}\n"), 26))?,
                );
            } else {
                body.extend_from_slice(
                    format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}")
                        .as_bytes(),
                );
            }

            body.extend_from_slice(b"\r\n");

            if body.len() > MAX_REQUEST_BODY_BYTES {
                return Err(ExecResult::err("curl: request body too large\n", 2));
            }
        }

        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

        Ok(body)
    }
}

struct CurlResponse {
    status: StatusCode,
    url: String,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl CurlResponse {
    fn write_out(&self, format: &str) -> Vec<u8> {
        format
            .replace("%%", "\0")
            .replace("\\n", "\n")
            .replace("\\t", "\t")
            .replace("%{http_code}", &self.status.as_u16().to_string())
            .replace("%{response_code}", &self.status.as_u16().to_string())
            .replace("%{url_effective}", &self.url)
            .replace("%{size_download}", &self.body.len().to_string())
            .replace(
                "%{content_type}",
                self.headers
                    .get(CONTENT_TYPE)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or(""),
            )
            .replace('\0', "%")
            .into_bytes()
    }

    fn headers(&self) -> Vec<u8> {
        let mut output = format!(
            "HTTP/1.1 {} {}\r\n",
            self.status.as_u16(),
            self.status
                .canonical_reason()
                .unwrap_or("")
        )
        .into_bytes();

        for (name, value) in &self.headers {
            output.extend_from_slice(name.as_str().as_bytes());
            output.extend_from_slice(b": ");
            output.extend_from_slice(value.as_bytes());
            output.extend_from_slice(b"\r\n");
        }

        output.extend_from_slice(b"\r\n");
        output
    }
}

pub struct Curl {
    client: HttpClient,
}

impl Curl {
    pub fn new(client: HttpClient) -> Self {
        Curl { client }
    }
}

#[async_trait]
impl Builtin for Curl {
    async fn execute(&self, context: BuiltinContext<'_>) -> bashkit::Result<ExecResult> {
        let mut options = match CurlOptions::parse(context.args) {
            Ok(options) => options,
            Err(result) => return Ok(result),
        };
        let mut body = match options.body(&context).await {
            Ok(body) => body,
            Err(result) => return Ok(result),
        };
        let mut url = match options.url.clone() {
            Some(url) => url,
            None => return Ok(ExecResult::err("curl: no URL specified\n", 3)),
        };

        if options.get
            && let Some(query) = body.take()
        {
            url.push(if url.contains('?') { '&' } else { '?' });
            match std::str::from_utf8(&query) {
                Ok(query) => url.push_str(query),
                Err(_) => return Ok(ExecResult::err("curl: invalid query bytes\n", 3)),
            }
        }

        let mut request = self
            .client
            .request(options.method.clone(), &url)
            .headers(options.headers.clone());

        if let Some(body) = body {
            request = request.body(body);
        }

        if let Some(timeout) = options.timeout {
            request = request.timeout(timeout);
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => return Ok(options.error(error)),
        };
        let status = response.status();
        let url = response.url().to_string();
        let headers = response.headers().clone();
        let body = match response.bytes().await {
            Ok(body) => body.to_vec(),
            Err(error) => return Ok(options.error(error)),
        };
        let response = CurlResponse { status, url, headers, body };

        if options.fail && response.status.as_u16() >= 400 {
            return Ok(ExecResult::err(
                if options.silent && !options.show_error {
                    String::new()
                } else {
                    format!("curl: server returned {}\n", response.status)
                },
                22,
            ));
        }

        let mut stdout = if options.head {
            response.headers()
        } else if options.output.is_some() || options.remote_name {
            if let Err(error) = context
                .fs
                .write_file(
                    &context
                        .cwd
                        .join(match options.output.as_deref() {
                            Some(path) => path,
                            None => match response
                                .url
                                .split('/')
                                .rfind(|segment| !segment.is_empty())
                            {
                                Some(filename) => filename,
                                None => {
                                    return Ok(ExecResult::err(
                                        "curl: remote URL has no filename\n",
                                        23,
                                    ));
                                }
                            },
                        }),
                    &response.body,
                )
                .await
            {
                return Ok(ExecResult::err(format!("curl: {error}\n"), 23));
            }

            Vec::new()
        } else {
            response.body.clone()
        };

        if let Some(format) = &options.write_out {
            stdout.extend_from_slice(&response.write_out(format));
        }

        Ok(ExecResult {
            stdout: stdout.into(),
            stderr: if options.verbose && !options.silent {
                format!("> {} {}\n< {}\n", options.method, response.url, response.status).into()
            } else {
                Vec::new().into()
            },
            exit_code: 0,
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use agentc_fs::fs::Fs;
    use agentc_http::{
        client::{
            HttpClient,
            policies::{PatternPolicy, UrlPattern},
        },
        protocol::Method,
    };
    use bashkit::{Bash, ExecResult};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        task::JoinHandle,
    };

    use crate::bash::{
        curl::{Curl, CurlOptions, DataKind, DataPart},
        fs::BashkitFs,
    };

    fn allowed_client(address: &str) -> HttpClient {
        HttpClient::builder()
            .policy(
                PatternPolicy::allow([UrlPattern::parse(format!("http://{address}/*")).unwrap()])
                    .unwrap(),
            )
            .build()
            .unwrap()
    }

    fn denied_client() -> HttpClient {
        HttpClient::builder()
            .policy(PatternPolicy::allow([]).unwrap())
            .build()
            .unwrap()
    }

    struct TestRequest {
        method: String,
        path: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    }

    impl TestRequest {
        async fn read(stream: &mut TcpStream) -> Self {
            let mut buffer = Vec::new();
            let mut chunk = [0u8; 1024];

            let header_end = loop {
                let read = stream.read(&mut chunk).await.unwrap();
                buffer.extend_from_slice(&chunk[..read]);

                if let Some(position) = buffer
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                {
                    break position;
                }
            };

            let head = String::from_utf8_lossy(&buffer[..header_end]).into_owned();
            let mut lines = head.split("\r\n");
            let mut request_parts = lines
                .next()
                .unwrap_or_default()
                .split(' ');
            let method = request_parts
                .next()
                .unwrap_or_default()
                .to_owned();
            let path = request_parts
                .next()
                .unwrap_or_default()
                .to_owned();
            let headers = lines
                .filter_map(|line| {
                    line.split_once(':')
                        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
                })
                .collect::<Vec<_>>();
            let content_length = headers
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .and_then(|(_, value)| value.parse::<usize>().ok())
                .unwrap_or(0);
            let mut body = buffer[header_end + 4..].to_vec();

            while body.len() < content_length {
                let read = stream.read(&mut chunk).await.unwrap();
                body.extend_from_slice(&chunk[..read]);
            }

            TestRequest { method, path, headers, body }
        }
    }

    struct TestResponse {
        status: u16,
        reason: &'static str,
        content_type: &'static str,
        body: &'static [u8],
        delay: Duration,
    }

    impl TestResponse {
        fn ok(body: &'static [u8]) -> Self {
            TestResponse {
                status: 200,
                reason: "OK",
                content_type: "text/plain",
                body,
                delay: Duration::ZERO,
            }
        }

        fn status(mut self, status: u16, reason: &'static str) -> Self {
            self.status = status;
            self.reason = reason;
            self
        }

        fn delayed_by(mut self, delay: Duration) -> Self {
            self.delay = delay;
            self
        }

        async fn write(&self, stream: &mut TcpStream) {
            tokio::time::sleep(self.delay).await;

            stream
                .write_all(
                    format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        self.status,
                        self.reason,
                        self.content_type,
                        self.body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            stream
                .write_all(self.body)
                .await
                .unwrap();
            stream.shutdown().await.unwrap();
        }
    }

    struct TestServer {
        address: String,
        handle: JoinHandle<Vec<TestRequest>>,
    }

    impl TestServer {
        async fn start(expected: usize, response: TestResponse) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .unwrap();
            let address = listener
                .local_addr()
                .unwrap()
                .to_string();

            let handle = tokio::spawn(async move {
                let mut requests = Vec::new();

                for _ in 0..expected {
                    let (mut stream, _) = listener.accept().await.unwrap();

                    requests.push(TestRequest::read(&mut stream).await);
                    response.write(&mut stream).await;
                }

                requests
            });

            TestServer { address, handle }
        }

        fn url(&self, path: &str) -> String {
            format!("http://{}{path}", self.address)
        }

        async fn requests(self) -> Vec<TestRequest> {
            self.handle.await.unwrap()
        }
    }

    struct TestShell {
        bash: Bash,
        fs: Fs,
    }

    impl TestShell {
        fn new(client: HttpClient) -> Self {
            let fs = Fs::memory();
            let bash = Bash::builder()
                .fs(Arc::new(BashkitFs::new(fs.clone())))
                .cwd("/")
                .builtin("curl", Box::new(Curl::new(client)))
                .build();

            TestShell { bash, fs }
        }

        fn allowing(server: &TestServer) -> Self {
            TestShell::new(allowed_client(&server.address))
        }

        async fn exec(&mut self, script: &str) -> ExecResult {
            self.bash.exec(script).await.unwrap()
        }

        async fn curl(&mut self, args: &str) -> ExecResult {
            self.exec(&format!("curl {args}")).await
        }

        async fn read(&self, path: &str) -> Vec<u8> {
            self.fs
                .root()
                .open_file(path)
                .await
                .unwrap()
                .read_to_end()
                .await
                .unwrap()
        }
    }

    #[test]
    fn implies_post_when_data_is_present_without_get_or_explicit_method() {
        let options = CurlOptions::parse(&[
            "http://example.test".to_owned(),
            "-d".to_owned(),
            "a=1".to_owned(),
        ])
        .ok()
        .unwrap();

        assert_eq!(options.method, Method::POST);
    }

    #[test]
    fn parses_headers_split_once_on_colon_trimming_only_the_value() {
        let options = CurlOptions::parse(&[
            "-H".to_owned(),
            "X-Test:  value".to_owned(),
            "http://example.test".to_owned(),
        ])
        .ok()
        .unwrap();

        assert_eq!(
            options
                .headers
                .get("X-Test")
                .unwrap()
                .to_str()
                .unwrap(),
            "value"
        );
    }

    #[test]
    fn rejects_a_missing_option_value() {
        assert!(CurlOptions::parse(&["-d".to_owned()]).is_err());
    }

    #[test]
    fn supports_compact_and_equals_data_forms() {
        let options = CurlOptions::parse(&["-dvalue".to_owned(), "http://example.test".to_owned()])
            .ok()
            .unwrap();

        assert!(matches!(options.data[0].kind, DataKind::Data));
        assert_eq!(options.data[0].value, "value");

        let options = CurlOptions::parse(&[
            "--data-raw=value".to_owned(),
            "http://example.test".to_owned(),
        ])
        .ok()
        .unwrap();

        assert!(matches!(options.data[0].kind, DataKind::Raw));
        assert_eq!(options.data[0].value, "value");
    }

    #[test]
    fn percent_encodes_reserved_bytes_and_spaces() {
        assert_eq!(DataPart::encode(b"a b/c"), b"a+b%2Fc".to_vec());
    }

    #[tokio::test]
    async fn missing_url_returns_exit_code_three() {
        let mut bash = Bash::builder()
            .builtin("curl", Box::new(Curl::new(HttpClient::builder().build().unwrap())))
            .build();

        let result = bash.exec("curl").await.unwrap();

        assert_eq!(result.exit_code, 3);
    }

    #[tokio::test]
    async fn get_returns_the_exact_response_body() {
        let server = TestServer::start(1, TestResponse::ok(b"hello")).await;
        let mut shell = TestShell::allowing(&server);

        let result = shell.curl(&server.url("/hello")).await;

        assert_eq!(result.stdout.as_bytes(), b"hello");
        assert_eq!(server.requests().await.len(), 1);
    }

    #[tokio::test]
    async fn post_with_headers_and_data_reaches_the_server() {
        let server = TestServer::start(1, TestResponse::ok(b"ok")).await;
        let mut shell = TestShell::allowing(&server);

        shell
            .curl(&format!(
                "-X POST -H 'X-One: a' -H 'X-Two: b' -d 'field=1' {}",
                server.url("/submit")
            ))
            .await;

        let requests = server.requests().await;

        assert_eq!(requests[0].method, "POST");
        assert_eq!(requests[0].path, "/submit");
        assert!(
            requests[0]
                .headers
                .iter()
                .any(|(name, value)| name.eq_ignore_ascii_case("x-one") && value == "a")
        );
        assert!(
            requests[0]
                .headers
                .iter()
                .any(|(name, value)| name.eq_ignore_ascii_case("x-two") && value == "b")
        );
        assert_eq!(requests[0].body, b"field=1");
    }

    #[tokio::test]
    async fn get_with_data_urlencode_appends_the_query() {
        let server = TestServer::start(1, TestResponse::ok(b"ok")).await;
        let mut shell = TestShell::allowing(&server);

        shell
            .curl(&format!("-G --data-urlencode 'q=a b' {}", server.url("/search")))
            .await;

        let requests = server.requests().await;

        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].path, "/search?q=a+b");
    }

    #[tokio::test]
    async fn data_at_file_reads_the_upload_from_the_injected_filesystem() {
        let server = TestServer::start(1, TestResponse::ok(b"ok")).await;
        let mut shell = TestShell::allowing(&server);

        shell
            .exec(&format!(
                "printf 'field=1' > /upload.txt && curl --data @/upload.txt {}",
                server.url("/submit")
            ))
            .await;

        assert_eq!(server.requests().await[0].body, b"field=1");
    }

    #[tokio::test]
    async fn data_binary_preserves_raw_bytes_including_newlines() {
        let server = TestServer::start(1, TestResponse::ok(b"ok")).await;
        let mut shell = TestShell::allowing(&server);

        shell
            .exec(&format!(
                "printf 'line1\\nline2' > /raw.bin && curl --data-binary @/raw.bin {}",
                server.url("/raw")
            ))
            .await;

        assert_eq!(server.requests().await[0].body, b"line1\nline2");
    }

    #[tokio::test]
    async fn multipart_form_upload_reads_the_injected_filesystem() {
        let server = TestServer::start(1, TestResponse::ok(b"ok")).await;
        let mut shell = TestShell::allowing(&server);

        shell
            .exec(&format!(
                "printf uploaded > /form.txt && curl -F file=@/form.txt -F name=agentc {}",
                server.url("/upload")
            ))
            .await;

        let requests = server.requests().await;
        let body = String::from_utf8_lossy(&requests[0].body);

        assert!(body.contains("name=\"file\"; filename=\"form.txt\""));
        assert!(body.contains("uploaded"));
        assert!(body.contains("name=\"name\""));
        assert!(body.contains("agentc"));
    }

    #[tokio::test]
    async fn output_flag_writes_the_response_body_to_the_injected_filesystem() {
        let server = TestServer::start(1, TestResponse::ok(b"downloaded")).await;
        let mut shell = TestShell::allowing(&server);

        let result = shell
            .curl(&format!("-o /download.txt {}", server.url("/file")))
            .await;

        assert!(result.stdout.as_bytes().is_empty());
        assert_eq!(shell.read("/download.txt").await, b"downloaded");
        server.requests().await;
    }

    #[tokio::test]
    async fn remote_name_flag_derives_the_filename_from_the_url() {
        let server = TestServer::start(1, TestResponse::ok(b"remote")).await;
        let mut shell = TestShell::allowing(&server);

        shell
            .curl(&format!("-O {}", server.url("/named.txt")))
            .await;

        assert_eq!(shell.read("/named.txt").await, b"remote");
        server.requests().await;
    }

    #[tokio::test]
    async fn head_flag_prints_headers_without_a_body() {
        let server = TestServer::start(1, TestResponse::ok(b"ignored")).await;
        let mut shell = TestShell::allowing(&server);

        let result = shell
            .curl(&format!("-I {}", server.url("/head")))
            .await;
        let stdout = result.stdout.text_lossy();

        assert!(stdout.starts_with("HTTP/1.1 200"));
        assert!(!stdout.contains("ignored"));
        server.requests().await;
    }

    #[tokio::test]
    async fn fail_flag_maps_an_http_error_status_to_exit_twenty_two() {
        let server =
            TestServer::start(1, TestResponse::ok(b"missing").status(404, "Not Found")).await;
        let mut shell = TestShell::allowing(&server);

        let result = shell
            .curl(&format!("-f {}", server.url("/missing")))
            .await;

        assert_eq!(result.exit_code, 22);
        assert!(result.stdout.as_bytes().is_empty());
        server.requests().await;
    }

    #[tokio::test]
    async fn write_out_appends_status_url_content_type_and_size() {
        let server = TestServer::start(1, TestResponse::ok(b"body")).await;
        let mut shell = TestShell::allowing(&server);

        let result = shell
            .curl(&format!(
                "-w '|%{{http_code}}|%{{url_effective}}|%{{content_type}}|%{{size_download}}' {}",
                server.url("/info")
            ))
            .await;
        let stdout = result.stdout.text_lossy();

        assert!(stdout.starts_with("body|200|"));
        assert!(stdout.contains(&server.url("/info")));
        assert!(stdout.ends_with("|text/plain|4"));
        server.requests().await;
    }

    #[tokio::test]
    async fn a_request_timeout_maps_to_exit_twenty_eight() {
        let server =
            TestServer::start(1, TestResponse::ok(b"late").delayed_by(Duration::from_millis(200)))
                .await;
        let mut shell = TestShell::allowing(&server);

        let result = shell
            .curl(&format!("-m 0.05 {}", server.url("/slow")))
            .await;

        assert_eq!(result.exit_code, 28);
        server.handle.abort();
    }

    #[tokio::test]
    async fn a_policy_denial_maps_to_exit_seven_without_contacting_the_server() {
        let server = TestServer::start(0, TestResponse::ok(b"unused")).await;
        let mut shell = TestShell::new(denied_client());

        let result = shell
            .curl(&server.url("/blocked"))
            .await;

        assert_eq!(result.exit_code, 7);
        assert!(server.requests().await.is_empty());
    }

    #[tokio::test]
    async fn a_response_size_limit_violation_maps_to_exit_sixty_three() {
        let server = TestServer::start(1, TestResponse::ok(b"this response is too large")).await;
        let client = HttpClient::builder()
            .policy(PatternPolicy::allow([UrlPattern::parse(server.url("/*")).unwrap()]).unwrap())
            .max_response_bytes(4u64)
            .build()
            .unwrap();
        let mut shell = TestShell::new(client);

        let result = shell.curl(&server.url("/big")).await;

        assert_eq!(result.exit_code, 63);
        server.requests().await;
    }
}
