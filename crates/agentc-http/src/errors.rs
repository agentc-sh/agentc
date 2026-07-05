// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use anyhow::Error;
use axum::extract::{
    path::ErrorKind,
    rejection::{JsonRejection, PathRejection, QueryRejection},
};
use serde::{Deserialize, Serialize};
use serde_json::to_value;
use validator::{ValidationError, ValidationErrors, ValidationErrorsKind};

use crate::dto::errors::{
    ErrorResponseDTO, ValidationErrorFieldDTO, ValidationErrorFieldDetailDTO,
};

fn flatten_validation_errors(
    errors: &ValidationErrors,
    prefix: Option<&str>,
) -> Vec<ValidationErrorFieldDTO> {
    let mut dtos = Vec::new();

    for (field_name, kind) in errors.errors() {
        let path = match prefix {
            Some(p) if !field_name.is_empty() => format!("{}.{}", p, field_name),
            Some(p) => p.to_string(),
            None => field_name.to_string(),
        };

        match kind {
            ValidationErrorsKind::Field(field_errors) => {
                dtos.push(ValidationErrorFieldDTO {
                    field: path,
                    errors: field_errors
                        .iter()
                        .map(|err| ValidationErrorFieldDetailDTO {
                            code: err.code.to_string(),
                            message: err
                                .message
                                .as_ref()
                                .map(|msg| msg.to_string()),
                            params: if err.params.is_empty() {
                                None
                            } else {
                                to_value(&err.params).ok()
                            },
                        })
                        .collect(),
                });
            }
            ValidationErrorsKind::Struct(struct_errors) => {
                dtos.extend(flatten_validation_errors(struct_errors, Some(&path)));
            }
            ValidationErrorsKind::List(list_errors) => {
                for (idx, idx_errors) in list_errors {
                    dtos.extend(flatten_validation_errors(
                        idx_errors,
                        Some(&format!("{}[{}]", path, idx)),
                    ));
                }
            }
        }
    }

    dtos
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ApiError {
    Generic { code: u32, message: String },
    Validation { code: u32, errors: ValidationErrors },
}

impl ApiError {
    pub fn new(code: u32, message: impl Into<String>) -> Self {
        Self::Generic { code, message: message.into() }
    }

    pub fn code(&self) -> u32 {
        match self {
            Self::Generic { code, .. } => *code,
            Self::Validation { code, .. } => *code,
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(403000, message)
    }

    pub fn unprocessable_entity(message: impl Into<String>) -> Self {
        Self::new(422000, message)
    }

    pub fn validation_errors(errors: ValidationErrors) -> Self {
        Self::Validation { code: 422001, errors }
    }

    pub fn validation_error(field: &'static str, error: ValidationError) -> Self {
        let mut errors = ValidationErrors::new();
        errors.add(field, error);
        Self::validation_errors(errors)
    }

    pub fn unexpected_error(message: impl Into<String>) -> Self {
        Self::new(500000, message)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(400000, message)
    }

    pub fn invalid_content_type(content_type: impl Into<String>) -> Self {
        Self::new(400001, format!("Invalid content type: {}", content_type.into()))
    }

    pub fn invalid_data_format(message: impl Into<String>) -> Self {
        Self::new(400002, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(404000, message)
    }

    pub fn not_implemented() -> Self {
        Self::new(500001, "Not implemented")
    }
}

impl From<Error> for ApiError {
    fn from(error: Error) -> Self {
        ApiError::unexpected_error(error.to_string())
    }
}

impl From<ValidationErrors> for ApiError {
    fn from(errors: ValidationErrors) -> Self {
        ApiError::validation_errors(errors)
    }
}

impl From<ApiError> for ErrorResponseDTO {
    fn from(error: ApiError) -> Self {
        match error {
            ApiError::Generic { code, message } => ErrorResponseDTO::Generic { code, message },
            ApiError::Validation { code, errors } => ErrorResponseDTO::Validation {
                code,
                fields: flatten_validation_errors(&errors, None),
            },
        }
    }
}

impl From<JsonRejection> for ApiError {
    fn from(error: JsonRejection) -> Self {
        match error {
            JsonRejection::JsonDataError(err) => ApiError::unprocessable_entity(err.to_string()),
            JsonRejection::JsonSyntaxError(_) => {
                ApiError::invalid_data_format("Invalid JSON syntax")
            }
            JsonRejection::MissingJsonContentType(_) => {
                ApiError::invalid_content_type("Missing JSON content type")
            }
            _ => ApiError::bad_request("Unexpected JSON rejection"),
        }
    }
}

impl From<PathRejection> for ApiError {
    fn from(error: PathRejection) -> Self {
        match error {
            PathRejection::FailedToDeserializePathParams(inner) => match inner.into_kind() {
                ErrorKind::WrongNumberOfParameters { got, expected } => ApiError::bad_request(
                    format!("Wrong number of path parameters: got {}, expected {}", got, expected),
                ),
                ErrorKind::ParseErrorAtKey { key, .. } => {
                    ApiError::bad_request(format!("Failed to parse path parameter '{}'", key))
                }
                ErrorKind::ParseErrorAtIndex { index, .. } => ApiError::bad_request(format!(
                    "Failed to parse path parameter at index {}",
                    index
                )),
                ErrorKind::ParseError { value, expected_type } => ApiError::bad_request(format!(
                    "Failed to parse path parameter value '{}' as {}",
                    value, expected_type
                )),
                ErrorKind::InvalidUtf8InPathParam { key } => {
                    ApiError::bad_request(format!("Invalid UTF-8 in path parameter '{}'", key))
                }
                ErrorKind::UnsupportedType { name } => {
                    ApiError::bad_request(format!("Unsupported type for path parameter '{}'", name))
                }
                ErrorKind::Message(msg) => ApiError::bad_request(&msg),
                ErrorKind::DeserializeError { message, .. } => {
                    ApiError::bad_request(message.as_str())
                }
                _ => ApiError::unexpected_error("Failed to extract path parameters"),
            },
            PathRejection::MissingPathParams(error) => {
                ApiError::unexpected_error(error.to_string())
            }
            _ => ApiError::unexpected_error("Failed to extract path parameters"),
        }
    }
}

impl From<QueryRejection> for ApiError {
    fn from(error: QueryRejection) -> Self {
        match error {
            QueryRejection::FailedToDeserializeQueryString(_) => {
                ApiError::bad_request("Invalid query string")
            }
            _ => ApiError::bad_request("Unexpected query rejection"),
        }
    }
}
