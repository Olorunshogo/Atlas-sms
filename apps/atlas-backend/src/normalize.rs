use axum::body::to_bytes;
use axum::extract::Request;
use axum::http::header::CONTENT_TYPE;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::error::ApiError;

/// Longest framework message worth echoing back; axum's rejection texts are a sentence or two.
const MAX_REJECTION_BYTES: usize = 8 * 1024;

/**
 * Turns anything the framework refuses before a handler runs into the API's own error shape.
 * A malformed JSON body, a bad id in a path or query string, a wrong content type, an unknown route or a wrong method all come back from axum as plain text or an empty body; clients that parse every failure as `{ message, success, code }` would choke on them.
 * Handler-produced errors are already JSON and pass through untouched.
 */
pub async fn json_errors(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    let status = response.status();
    let is_json = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.starts_with("application/json"))
        .unwrap_or(false);
    if !status.is_client_error() || is_json {
        return response;
    }

    let (_, body) = response.into_parts();
    let bytes = to_bytes(body, MAX_REJECTION_BYTES).await.unwrap_or_default();
    let text = String::from_utf8_lossy(&bytes).trim().to_string();
    let (message, code) = match status {
        StatusCode::NOT_FOUND => ("Route not found.".to_string(), "ROUTE_NOT_FOUND"),
        StatusCode::METHOD_NOT_ALLOWED => ("Method not allowed.".to_string(), "METHOD_NOT_ALLOWED"),
        _ if text.is_empty() => (status.canonical_reason().unwrap_or("Bad request").to_string(), "INVALID_REQUEST"),
        _ => (text, "INVALID_REQUEST"),
    };
    return ApiError::rejected(status, message, code).into_response();
}
