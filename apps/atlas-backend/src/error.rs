use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

// === ApiError

/**
 * Every handler error path funnels through this type so the response shape
 * (`{ message, success, code }`) stays consistent across the whole API.
 */
#[derive(Debug)]
pub enum ApiError {
    BadRequest {
        message: String,
        code: Option<&'static str>,
    },
    Unauthorized(String),
    Forbidden {
        message: String,
        code: Option<&'static str>,
    },
    NotFound(String),
    Conflict {
        message: String,
        code: Option<&'static str>,
    },
    Internal(String),
    /// A request the framework refused before a handler ran (bad JSON, bad
    /// path, unknown route); keeps the status axum chose.
    Rejected {
        status: StatusCode,
        message: String,
        code: &'static str,
    },
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest {
            message: message.into(),
            code: None,
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden {
            message: message.into(),
            code: None,
        }
    }

    pub fn rejected(status: StatusCode, message: String, code: &'static str) -> Self {
        Self::Rejected {
            status,
            message,
            code,
        }
    }

    pub fn bad_request_with_code(message: impl Into<String>, code: &'static str) -> Self {
        Self::BadRequest {
            message: message.into(),
            code: Some(code),
        }
    }

    pub fn conflict_with_code(message: impl Into<String>, code: &'static str) -> Self {
        Self::Conflict {
            message: message.into(),
            code: Some(code),
        }
    }
}

#[derive(Serialize)]
struct ErrorBody {
    message: String,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<&'static str>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message, code) = match self {
            ApiError::BadRequest { message, code } => (StatusCode::BAD_REQUEST, message, code),
            ApiError::Unauthorized(message) => (StatusCode::UNAUTHORIZED, message, None),
            ApiError::Forbidden { message, code } => (StatusCode::FORBIDDEN, message, code),
            ApiError::NotFound(message) => (StatusCode::NOT_FOUND, message, None),
            ApiError::Conflict { message, code } => (StatusCode::CONFLICT, message, code),
            ApiError::Rejected {
                status,
                message,
                code,
            } => (status, message, Some(code)),
            ApiError::Internal(message) => {
                eprintln!("internal error: {message}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong".to_string(),
                    None,
                )
            }
        };

        (
            status,
            Json(ErrorBody {
                message,
                success: false,
                code,
            }),
        )
            .into_response()
    }
}

/**
 * Constraint violations are the caller's mistake, not a server fault, so
 * they become 4xx instead of a blanket 500. The message is generic on
 * purpose: constraint and column names are database internals.
 */
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db_err) = &err {
            match db_err.code().as_deref() {
                // unique_violation
                Some("23505") => {
                    return ApiError::conflict_with_code(
                        "A record with these details already exists.",
                        "ALREADY_EXISTS",
                    )
                }
                // foreign_key_violation
                Some("23503") => {
                    return ApiError::bad_request_with_code(
                        "A referenced record does not exist.",
                        "INVALID_REFERENCE",
                    )
                }
                // check_violation
                Some("23514") => {
                    return ApiError::bad_request_with_code(
                        "A value is out of the allowed range.",
                        "INVALID_VALUE",
                    )
                }
                _ => {}
            }
        }
        ApiError::Internal(err.to_string())
    }
}

impl From<argon2::password_hash::Error> for ApiError {
    fn from(err: argon2::password_hash::Error) -> Self {
        ApiError::Internal(format!("password hashing failure: {err}"))
    }
}
