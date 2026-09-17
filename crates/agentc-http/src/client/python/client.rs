// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{collections::HashSet, marker::PhantomData, time::Duration};

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{
        errors::Error,
        handle::{Class, Instance, Object},
        host::{
            exception::{ExceptionClass, Raise},
            state::ModuleState,
        },
        host_class,
        scope::Enter,
    },
};
use bytes::Bytes;
use http::{HeaderMap, Method, header::CONTENT_TYPE};
use url::{ParseError, Url};

use crate::client::{
    client::HttpClient,
    errors::HttpClientError,
    python::{
        body::RequestBody,
        exchange::Exchange,
        headers::{HeaderTypes, Headers},
        module::HttpModule,
        params::QueryParams,
        pending::PendingResponse,
        request::Request,
        timeout::FromSeconds,
        upload::Upload,
    },
    request::HttpRequestBuilder,
};

struct Prepared<B: ExecutorBackend> {
    request: HttpRequestBuilder,
    upload: Option<Upload<B>>,
}

pub struct Client<B: ExecutorBackend> {
    http: HttpClient,
    base_url: Option<String>,
    headers: HeaderMap,
    params: Vec<(String, String)>,
    timeout: Option<Duration>,
    _marker: PhantomData<fn() -> B>,
}

impl<B: ExecutorBackend> Client<B> {
    fn base_url(base_url: Option<String>) -> Result<Option<String>, Error> {
        let Some(base_url) = base_url else {
            return Ok(None);
        };
        let mut parsed = Url::parse(&base_url).map_err(|_| {
            Error::from(
                Raise::<B>::new(ExceptionClass::builtin("ValueError"))
                    .arg("base_url must be an absolute URL without a query string or fragment"),
            )
        })?;

        if base_url.is_empty()
            || parsed.cannot_be_a_base()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(Raise::<B>::new(ExceptionClass::builtin("ValueError"))
                .arg("base_url must be an absolute URL without a query string or fragment")
                .into());
        }

        if !parsed.path().ends_with('/') {
            parsed.set_path(&format!("{}/", parsed.path(),));
        }

        Ok(Some(parsed.to_string()))
    }

    fn url(&self, request: &str) -> Result<Url, HttpClientError> {
        match (&self.base_url, Url::parse(request)) {
            (None, Ok(url)) => Ok(url),
            (None, Err(ParseError::RelativeUrlWithoutBase)) => {
                Err(HttpClientError::invalid_request("a relative URL needs a base_url"))
            }
            (None, Err(error)) => Err(HttpClientError::invalid_request(error.to_string())),
            (Some(_), Ok(_)) => Err(HttpClientError::invalid_request(
                "an absolute URL cannot be used with base_url",
            )),
            (Some(base_url), Err(_)) => Url::parse(base_url)
                .expect("validated base URL must remain valid")
                .join(request.trim_start_matches('/'))
                .map_err(|error| HttpClientError::invalid_request(error.to_string())),
        }
    }

    fn query(&self, url: &mut Url, request: &[(String, String)]) {
        if self.params.is_empty() && request.is_empty() {
            return;
        }

        let overrides = request
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<HashSet<_>>();

        url.query_pairs_mut()
            .extend_pairs(
                self.params
                    .iter()
                    .filter(|(name, _)| !overrides.contains(name.as_str())),
            )
            .extend_pairs(request);
    }

    fn headers(&self, request: &HeaderMap) -> HeaderMap {
        let mut headers = self.headers.clone();

        for name in request.keys() {
            headers.remove(name);
        }

        for (name, value) in request {
            headers.append(name, value.clone());
        }

        headers
    }

    fn prepare(&self, request: &Request<B>, exchange: &Exchange<B>) -> Result<Prepared<B>, Error> {
        let mut url = self
            .url(&request.url)
            .map_err(|error| exchange.raise(error))?;

        self.query(&mut url, &request.params);

        let mut headers = self.headers(&request.headers);

        if !headers.contains_key(CONTENT_TYPE) {
            if let Some(content_type) = request
                .body
                .as_ref()
                .and_then(RequestBody::content_type)
            {
                headers.insert(
                    CONTENT_TYPE,
                    content_type
                        .parse()
                        .expect("static content type"),
                );
            }
        }

        let mut prepared = Prepared {
            request: self
                .http
                .request(request.method.clone(), url.as_str())
                .headers(headers),
            upload: None,
        };

        if let Some(body) = &request.body {
            match body {
                RequestBody::Bytes(body) => {
                    prepared.request = prepared.request.body(body.clone());
                }
                RequestBody::Text(body) => {
                    prepared.request = prepared
                        .request
                        .body(Bytes::copy_from_slice(body.as_bytes()));
                }
                RequestBody::Json(body) => {
                    prepared.request =
                        prepared
                            .request
                            .body(Bytes::from(serde_json::to_vec(&body).map_err(|error| {
                                exchange.raise(HttpClientError::invalid_request(error.to_string()))
                            })?));
                }
                RequestBody::Form(body) => {
                    prepared.request = prepared.request.body(Bytes::from(
                        serde_urlencoded::to_string(body).map_err(|error| {
                            exchange.raise(HttpClientError::invalid_request(error.to_string()))
                        })?,
                    ));
                }
                RequestBody::Stream(body) => {
                    let (upload, receiver) = Upload::new(body)?;

                    prepared.request = prepared.request.stream(receiver);
                    prepared.upload = Some(upload);
                }
            }
        }

        if let Some(timeout) = request.timeout.or(self.timeout) {
            prepared.request = prepared.request.timeout(timeout);
        }

        Ok(prepared)
    }

    fn send_with_method(
        &self,
        enter: &Enter<'_, B>,
        method: Method,
        url: String,
        params: Option<Object<B>>,
        headers: Option<Object<B>>,
        body: Option<Object<B>>,
        timeout: Option<Object<B>>,
    ) -> Result<PendingResponse<B>, Error> {
        let mut kwargs = Vec::new();

        if let Some(params) = params {
            kwargs.push((String::from("params"), params));
        }

        if let Some(headers) = headers {
            kwargs.push((String::from("headers"), headers));
        }

        if let Some(body) = body {
            kwargs.push((String::from("body"), body));
        }

        if let Some(timeout) = timeout {
            kwargs.push((String::from("timeout"), timeout));
        }

        self.send(
            Class::of::<Request<B>>(enter)?
                .construct_with((method.as_str().to_owned(), url), kwargs)?,
        )
    }
}

#[host_class(backend = B, crate_path = agentc_executor_python::guestpy)]
impl<B: ExecutorBackend> Client<B> {
    #[guestpy(constructor)]
    fn new(
        #[guestpy(context)] module: ModuleState<HttpModule>,
        #[guestpy(kw)] base_url: Option<String>,
        #[guestpy(kw)] headers: Option<HeaderTypes<B>>,
        #[guestpy(kw)] params: Option<QueryParams>,
        #[guestpy(kw)] timeout: Option<f64>,
    ) -> Result<Self, Error> {
        Ok(Self {
            http: module.client().clone(),
            base_url: Self::base_url(base_url)?,
            headers: match headers {
                Some(headers) => Headers::map(headers)?,
                None => HeaderMap::new(),
            },
            params: params
                .map(QueryParams::pairs)
                .unwrap_or_default(),
            timeout: timeout
                .map(Duration::from_seconds::<B>)
                .transpose()?,
            _marker: PhantomData,
        })
    }

    #[guestpy(method)]
    fn send(&self, request: Instance<B, Request<B>>) -> Result<PendingResponse<B>, Error> {
        let exchange = Exchange::Sending { request: request.clone() };
        let prepared = request.borrow_with(|value| self.prepare(value, &exchange))??;

        Ok(PendingResponse::new(prepared.request, request, prepared.upload))
    }

    #[guestpy(method)]
    fn get(
        &self,
        #[guestpy(enter)] enter: &Enter<'_, B>,
        url: String,
        #[guestpy(kw)] params: Option<Object<B>>,
        #[guestpy(kw)] headers: Option<Object<B>>,
        #[guestpy(kw)] timeout: Option<Object<B>>,
    ) -> Result<PendingResponse<B>, Error> {
        self.send_with_method(enter, Method::GET, url, params, headers, None, timeout)
    }

    #[guestpy(method)]
    fn head(
        &self,
        #[guestpy(enter)] enter: &Enter<'_, B>,
        url: String,
        #[guestpy(kw)] params: Option<Object<B>>,
        #[guestpy(kw)] headers: Option<Object<B>>,
        #[guestpy(kw)] timeout: Option<Object<B>>,
    ) -> Result<PendingResponse<B>, Error> {
        self.send_with_method(enter, Method::HEAD, url, params, headers, None, timeout)
    }

    #[guestpy(method)]
    fn options(
        &self,
        #[guestpy(enter)] enter: &Enter<'_, B>,
        url: String,
        #[guestpy(kw)] params: Option<Object<B>>,
        #[guestpy(kw)] headers: Option<Object<B>>,
        #[guestpy(kw)] timeout: Option<Object<B>>,
    ) -> Result<PendingResponse<B>, Error> {
        self.send_with_method(enter, Method::OPTIONS, url, params, headers, None, timeout)
    }

    #[guestpy(method)]
    fn delete(
        &self,
        #[guestpy(enter)] enter: &Enter<'_, B>,
        url: String,
        #[guestpy(kw)] params: Option<Object<B>>,
        #[guestpy(kw)] headers: Option<Object<B>>,
        #[guestpy(kw)] body: Option<Object<B>>,
        #[guestpy(kw)] timeout: Option<Object<B>>,
    ) -> Result<PendingResponse<B>, Error> {
        self.send_with_method(enter, Method::DELETE, url, params, headers, body, timeout)
    }

    #[guestpy(method)]
    fn post(
        &self,
        #[guestpy(enter)] enter: &Enter<'_, B>,
        url: String,
        #[guestpy(kw)] params: Option<Object<B>>,
        #[guestpy(kw)] headers: Option<Object<B>>,
        #[guestpy(kw)] body: Option<Object<B>>,
        #[guestpy(kw)] timeout: Option<Object<B>>,
    ) -> Result<PendingResponse<B>, Error> {
        self.send_with_method(enter, Method::POST, url, params, headers, body, timeout)
    }

    #[guestpy(method)]
    fn put(
        &self,
        #[guestpy(enter)] enter: &Enter<'_, B>,
        url: String,
        #[guestpy(kw)] params: Option<Object<B>>,
        #[guestpy(kw)] headers: Option<Object<B>>,
        #[guestpy(kw)] body: Option<Object<B>>,
        #[guestpy(kw)] timeout: Option<Object<B>>,
    ) -> Result<PendingResponse<B>, Error> {
        self.send_with_method(enter, Method::PUT, url, params, headers, body, timeout)
    }

    #[guestpy(method)]
    fn patch(
        &self,
        #[guestpy(enter)] enter: &Enter<'_, B>,
        url: String,
        #[guestpy(kw)] params: Option<Object<B>>,
        #[guestpy(kw)] headers: Option<Object<B>>,
        #[guestpy(kw)] body: Option<Object<B>>,
        #[guestpy(kw)] timeout: Option<Object<B>>,
    ) -> Result<PendingResponse<B>, Error> {
        self.send_with_method(enter, Method::PATCH, url, params, headers, body, timeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use agentc_executor_python::guestpy::rustpython::RustPython;

    fn client<B: ExecutorBackend>(base_url: Option<&str>) -> Client<B> {
        Client {
            http: HttpClient::builder()
                .build()
                .expect("client builds"),
            base_url: base_url.map(str::to_owned),
            headers: HeaderMap::new(),
            params: Vec::new(),
            timeout: None,
            _marker: PhantomData,
        }
    }

    #[test]
    fn url_composition_is_scoped_by_the_base() {
        assert_eq!(
            client::<RustPython>(None)
                .url("https://other.example/x")
                .expect("absolute URL is valid")
                .as_str(),
            "https://other.example/x",
        );
        assert_eq!(
            client::<RustPython>(Some("https://api.example.com/v1/"),)
                .url("/users")
                .expect("relative URL is valid")
                .as_str(),
            "https://api.example.com/v1/users",
        );
        assert!(
            client::<RustPython>(None)
                .url("/users")
                .is_err(),
        );
        assert!(
            client::<RustPython>(Some("https://api.example.com/v1/"),)
                .url("https://other.example/x")
                .is_err(),
        );
    }

    #[test]
    fn request_query_values_replace_defaults_and_follow_the_url_query() {
        let mut client = client::<RustPython>(None);
        client.params = vec![
            (String::from("a"), String::from("1")),
            (String::from("b"), String::from("2")),
        ];
        let mut url = Url::parse("https://example.test/users?page=2").expect("test URL parses");

        client.query(
            &mut url,
            &[
                (String::from("b"), String::from("3")),
                (String::from("c"), String::from("4")),
            ],
        );

        assert_eq!(url.query(), Some("page=2&a=1&b=3&c=4"));
    }

    #[test]
    fn request_headers_replace_every_default_value_for_the_name() {
        let mut client = client::<RustPython>(None);
        client
            .headers
            .append("accept", "text/plain".parse().unwrap());
        client
            .headers
            .append("accept", "text/html".parse().unwrap());
        client
            .headers
            .insert("x-default", "yes".parse().unwrap());
        let mut request = HeaderMap::new();
        request.append("accept", "application/json".parse().unwrap());
        request.append(
            "accept",
            "application/problem+json"
                .parse()
                .unwrap(),
        );

        let headers = client.headers(&request);

        assert_eq!(headers.get_all("accept").iter().count(), 2);
        assert_eq!(headers.get("x-default").unwrap(), "yes");
    }
}
