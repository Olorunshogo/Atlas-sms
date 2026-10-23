use axum::body::Bytes;
use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::header::CONTENT_TYPE;
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::ApiError;

// === Json

/**
 * Drop-in for `axum::Json` that reports a bad body in the API's own error
 * shape (`{ message, success, code }`) instead of axum's plain-text
 * rejection. Also serializes responses exactly like `axum::Json`.
 */
pub struct Json<T>(pub T);

impl<S, T> FromRequest<S> for Json<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();
        if !is_json_content_type(&parts.headers) {
            return Err(ApiError::rejected(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "Expected request with `Content-Type: application/json`".to_string(),
                "INVALID_BODY",
            ));
        }
        let bytes = Bytes::from_request(Request::from_parts(parts, body), state)
            .await
            .map_err(|rejection| {
                ApiError::rejected(rejection.status(), rejection.body_text(), "INVALID_BODY")
            })?;

        /*
         * Every request body in this API is an object. serde would
         * otherwise happily read a JSON array into a struct field by field
         * (`["a","b"]` becoming `{ name: "a", slug: "b" }`), which no client
         * means to send.
         */
        if bytes.iter().find(|byte| !byte.is_ascii_whitespace()) == Some(&b'{') {
            axum::Json::<T>::from_bytes(&bytes)
                .map(|axum::Json(value)| Json(value))
                .map_err(json_rejection)
        } else {
            // Valid JSON that is not an object (422) versus not JSON at all (400).
            let status = match serde_json::from_slice::<serde::de::IgnoredAny>(&bytes) {
                Ok(_) => StatusCode::UNPROCESSABLE_ENTITY,
                Err(_) => StatusCode::BAD_REQUEST,
            };
            Err(ApiError::rejected(
                status,
                "Expected a JSON object request body".to_string(),
                "INVALID_BODY",
            ))
        }
    }
}

fn is_json_content_type(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            let media_type = value
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            media_type == "application/json"
                || (media_type.starts_with("application/") && media_type.ends_with("+json"))
        })
        .unwrap_or(false)
}

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

fn json_rejection(rejection: JsonRejection) -> ApiError {
    ApiError::rejected(rejection.status(), rejection.body_text(), "INVALID_BODY")
}

// === Path

pub struct Path<T>(pub T);

impl<S, T> FromRequestParts<S> for Path<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        axum::extract::Path::<T>::from_request_parts(parts, state)
            .await
            .map(|axum::extract::Path(value)| Path(value))
            .map_err(|rejection: PathRejection| {
                ApiError::rejected(rejection.status(), rejection.body_text(), "INVALID_PATH")
            })
    }
}

// === Query

pub struct Query<T>(pub T);

impl<S, T> FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        axum::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map(|axum::extract::Query(value)| Query(value))
            .map_err(|rejection: QueryRejection| {
                ApiError::rejected(rejection.status(), rejection.body_text(), "INVALID_QUERY")
            })
    }
}

// === Fallbacks

/// Unknown route, in the API's error shape.
pub async fn not_found() -> ApiError {
    ApiError::NotFound("Route not found.".to_string())
}

/// Known route, wrong HTTP method.
pub async fn method_not_allowed() -> ApiError {
    ApiError::rejected(
        StatusCode::METHOD_NOT_ALLOWED,
        "Method not allowed.".to_string(),
        "METHOD_NOT_ALLOWED",
    )
}
