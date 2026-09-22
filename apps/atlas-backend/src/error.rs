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

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        ApiError::Internal(err.to_string())
    }
}

impl From<argon2::password_hash::Error> for ApiError {
    fn from(err: argon2::password_hash::Error) -> Self {
        ApiError::Internal(format!("password hashing failure: {err}"))
    }
}
