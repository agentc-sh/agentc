// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use axum::{
    extract::{FromRequest, FromRequestParts, OptionalFromRequestParts, Request},
    http::request::Parts,
    response::{IntoResponse, Response},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_qs::Config as QsConfig;
use utoipa::ToSchema;

use crate::{dto::errors::ErrorResponseDTO, errors::ApiError};

#[derive(Debug, Clone, ToSchema)]
pub struct TenantIdHeader(pub String);

impl TenantIdHeader {
    pub fn new(tenant_id: String) -> Self {
        Self(tenant_id)
    }

    pub fn as_inner(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }

    pub fn extract_from_parts(parts: &Parts) -> Option<TenantIdHeader> {
        match parts.headers.get("x-tenant-id") {
            Some(value) => Some(Self(value.to_str().ok()?.to_string())),
            None => None,
        }
    }
}

/// Tenant ID header extractor.
impl<S> FromRequestParts<S> for TenantIdHeader
where
    S: Send + Sync,
{
    type Rejection = ErrorResponseDTO;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        match Self::extract_from_parts(parts) {
            Some(tenant_id) => Ok(tenant_id),
            None => {
                Err(ErrorResponseDTO::from(ApiError::bad_request("Missing X-Tenant-ID header")))
            }
        }
    }
}

/// Optional tenant ID header extractor.
impl<S> OptionalFromRequestParts<S> for TenantIdHeader
where
    S: Send + Sync,
{
    type Rejection = ErrorResponseDTO;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(Self::extract_from_parts(parts))
    }
}

/// Path parameters extractor with custom error handling.
#[derive(Debug, Clone, ToSchema)]
pub struct Path<T>(pub T);

impl<S, T> FromRequestParts<S> for Path<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = ErrorResponseDTO;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match <axum::extract::Path<T> as FromRequestParts<S>>::from_request_parts(parts, state)
            .await
        {
            Ok(value) => Ok(Self(value.0)),
            Err(rejection) => Err(ErrorResponseDTO::from(ApiError::from(rejection))),
        }
    }
}

/// Query parameters extractor that handles vectors and nested objects.
///
/// # Example
/// ```rust,ignore
/// #[derive(Deserialize, Serialize, Debug, IntoParams)]
/// struct QueryParams {
///     id: Option<String>,
///     name: Option<String>,
///     tags: Vec<String>,
///     labels: HashMap<String, String>,
/// }
/// ```
///
/// The above would be deserialized from the following query string:
/// ```ignore
/// ?id=123&name=John&tags[]=tag1&tags[]=tag2&labels[key1]=value1&labels[key2]=value2
/// ```
///
#[derive(Debug, Clone, ToSchema)]
pub struct Query<T>(pub T);

impl<S, T> FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = ErrorResponseDTO;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        QsConfig::new()
            .max_depth(5)
            .use_form_encoding(false)
            .deserialize_str::<T>(parts.uri.query().unwrap_or(""))
            .map(|query| Query(query))
            .map_err(|err| ErrorResponseDTO::from(ApiError::bad_request(err.to_string().as_str())))
    }
}

/// Optional query parameters extractor that handles vectors and nested objects.
///
/// # Example
/// ```rust,ignore
/// #[derive(Deserialize, Serialize, Debug, IntoParams)]
/// struct QueryParams {
///     id: Option<String>,
///     name: Option<String>,
///     tags: Vec<String>,
///     labels: HashMap<String, String>,
/// }
/// ```
///
/// The above would be deserialized from the following query string:
/// ```ignore
/// ?id=123&name=John&tags[]=tag1&tags[]=tag2&labels[key1]=value1&labels[key2]=value2
/// ```
///
#[derive(Debug, Clone, ToSchema)]
pub struct OptionalQuery<T>(pub Option<T>);

impl<S, T> FromRequestParts<S> for OptionalQuery<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = ErrorResponseDTO;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match parts.uri.query() {
            Some(_) => Query::from_request_parts(parts, state)
                .await
                .map(|q| OptionalQuery(Some(q.0))),
            None => Ok(OptionalQuery(None)),
        }
    }
}

/// JSON body extractor with custom error handling.
pub struct Json<T>(pub T);

impl<S, T> FromRequest<S> for Json<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = ErrorResponseDTO;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        match <axum::extract::Json<T> as FromRequest<S>>::from_request(request, state).await {
            Ok(value) => Ok(Self(value.0)),
            Err(rejection) => Err(ErrorResponseDTO::from(ApiError::from(rejection))),
        }
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        <axum::response::Json<T>>::from(self.0).into_response()
    }
}
