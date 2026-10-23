mod common;

use axum::http::{Method, StatusCode};
use common::{bare_world, send_raw, world, World};
use serde_json::{json, Value};
use uuid::Uuid;

/**
 * Every failure must be `{ message: string, success: false, code?: string }`
 * served as JSON, whether a handler produced it or the framework refused the
 * request before any handler ran.
 */
fn assert_api_error(
    response: (StatusCode, Option<String>, Vec<u8>),
    status: StatusCode,
    code: &str,
) -> Value {
    let (actual, content_type, bytes) = response;
    let text = String::from_utf8_lossy(&bytes).to_string();
    assert_eq!(actual, status, "{text}");
    assert!(
        content_type
            .as_deref()
            .is_some_and(|value| value.starts_with("application/json")),
        "expected JSON, got {content_type:?}: {text}"
    );
    let body: Value = serde_json::from_slice(&bytes).unwrap_or_else(|_| panic!("not JSON: {text}"));
    assert_eq!(body["success"], false, "{body}");
    assert!(
        body["message"].as_str().is_some_and(|m| !m.is_empty()),
        "{body}"
    );
    if code.is_empty() {
        assert!(body["code"].is_null(), "{body}");
    } else {
        assert_eq!(body["code"], code, "{body}");
    }
    body
}

async fn raw(
    w: &World,
    method: Method,
    uri: &str,
    content_type: Option<&str>,
    body: &str,
) -> (StatusCode, Option<String>, Vec<u8>) {
    send_raw(&w.app, method, uri, Some(&w.token), content_type, body).await
}

// === Bad request bodies

#[tokio::test]
async fn malformed_json_is_a_json_400() {
    let w = bare_world().await;
    for broken in [
        "{ not json",
        "",
        "   ",
        "{\"name\": \"A\"",
        "{\"name\": }",
        "nope",
    ] {
        let response = raw(
            &w,
            Method::POST,
            "/api/v1/stores",
            Some("application/json"),
            broken,
        )
        .await;
        assert_api_error(response, StatusCode::BAD_REQUEST, "INVALID_BODY");
    }
}

#[tokio::test]
async fn a_body_of_the_wrong_shape_is_a_json_422() {
    let w = bare_world().await;
    let cases = [
        // Arrays and scalars are not objects, even though serde could read an array into a struct.
        r#"["not", "an", "object"]"#,
        r#""just a string""#,
        "42",
        "null",
        r#"{"name": "No slug"}"#,
        r#"{"name": 42, "slug": "x"}"#,
        r#"{"name": "A", "slug": null}"#,
    ];
    for body in cases {
        let response = raw(
            &w,
            Method::POST,
            "/api/v1/stores",
            Some("application/json"),
            body,
        )
        .await;
        assert_api_error(response, StatusCode::UNPROCESSABLE_ENTITY, "INVALID_BODY");
    }
    // Nothing was created by any of them.
    let (_, stores) = w.get("/api/v1/stores").await;
    assert!(stores["data"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn a_missing_or_wrong_content_type_is_a_json_415() {
    let w = bare_world().await;
    for content_type in [
        None,
        Some("text/plain"),
        Some("application/x-www-form-urlencoded"),
    ] {
        let response = raw(
            &w,
            Method::POST,
            "/api/v1/stores",
            content_type,
            r#"{"name":"A","slug":"a"}"#,
        )
        .await;
        assert_api_error(response, StatusCode::UNSUPPORTED_MEDIA_TYPE, "INVALID_BODY");
    }
}

#[tokio::test]
async fn unknown_and_immutable_fields_are_a_json_422_that_names_the_field() {
    let w = world().await;
    let uri = format!("/api/v1/stores/{}", w.stores[0]);
    let response = raw(
        &w,
        Method::PATCH,
        &uri,
        Some("application/json"),
        r#"{"currency":"USD"}"#,
    )
    .await;
    let body = assert_api_error(response, StatusCode::UNPROCESSABLE_ENTITY, "INVALID_BODY");
    assert!(
        body["message"].as_str().unwrap().contains("currency"),
        "{body}"
    );
}

// === Bad paths and queries

#[tokio::test]
async fn a_malformed_id_in_the_path_is_a_json_400() {
    let w = world().await;
    for (method, uri) in [
        (Method::GET, "/api/v1/stores/not-a-uuid"),
        (Method::GET, "/api/v1/products/12345"),
        (Method::GET, "/api/v1/stock/products/abc"),
        (Method::GET, "/api/v1/sales/xyz"),
        (Method::POST, "/api/v1/sales/xyz/void"),
        (Method::DELETE, "/api/v1/price-rules/nope"),
    ] {
        let response = raw(
            &w,
            method.clone(),
            uri,
            Some("application/json"),
            r#"{"reason":"r"}"#,
        )
        .await;
        assert_api_error(response, StatusCode::BAD_REQUEST, "INVALID_PATH");
    }
}

#[tokio::test]
async fn a_malformed_query_is_a_json_400() {
    let w = world().await;
    for uri in [
        "/api/v1/stock/on-hand?store_id=abc",
        "/api/v1/stock/on-hand",
        "/api/v1/stock/inventory?low_stock=maybe",
        "/api/v1/stock/movements?limit=lots",
        "/api/v1/sales?shift_id=1",
    ] {
        let response = raw(&w, Method::GET, uri, None, "").await;
        assert_api_error(response, StatusCode::BAD_REQUEST, "INVALID_QUERY");
    }
}

// === Routing

#[tokio::test]
async fn unknown_routes_and_wrong_methods_are_json() {
    let w = world().await;
    for uri in [
        "/api/v1/nothing-here",
        "/api/v1/stock/unknown",
        "/elsewhere",
    ] {
        assert_api_error(
            raw(&w, Method::GET, uri, None, "").await,
            StatusCode::NOT_FOUND,
            "",
        );
    }
    for (method, uri) in [
        (Method::DELETE, "/api/v1/stores"),
        (Method::PUT, "/api/v1/stock/restocks"),
        (Method::GET, "/api/v1/stock/transfers"),
    ] {
        assert_api_error(
            raw(&w, method, uri, None, "").await,
            StatusCode::METHOD_NOT_ALLOWED,
            "METHOD_NOT_ALLOWED",
        );
    }
}

#[tokio::test]
async fn unauthenticated_requests_are_a_json_401() {
    let w = bare_world().await;
    for (token, expected) in [(None, "missing"), (Some("garbage"), "Invalid")] {
        let (status, content_type, bytes) =
            send_raw(&w.app, Method::GET, "/api/v1/stores", token, None, "").await;
        let body = assert_api_error((status, content_type, bytes), StatusCode::UNAUTHORIZED, "");
        assert!(
            body["message"].as_str().unwrap().contains(expected),
            "{body}"
        );
    }
}

// === The handlers' own errors keep the same shape

#[tokio::test]
async fn handler_errors_share_the_shape() {
    let w = world().await;
    w.seed_rice().await;
    let cases: Vec<(&str, Method, String, Value, StatusCode, &str)> = vec![
        (
            "validation",
            Method::POST,
            "/api/v1/stores".into(),
            json!({ "name": "", "slug": "x" }),
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
        ),
        (
            "conflict",
            Method::POST,
            "/api/v1/stores".into(),
            json!({ "name": "A", "slug": "location-1" }),
            StatusCode::CONFLICT,
            "ALREADY_EXISTS",
        ),
        (
            "insufficient stock",
            Method::POST,
            "/api/v1/stock/transfers".into(),
            json!({ "from_store_id": w.stores[0], "to_store_id": w.stores[1], "product_id": w.rice, "quantity": 9999, "reason": "r" }),
            StatusCode::CONFLICT,
            "INSUFFICIENT_STOCK",
        ),
    ];
    for (label, method, uri, body, status, code) in cases {
        let response = raw(
            &w,
            method,
            &uri,
            Some("application/json"),
            &body.to_string(),
        )
        .await;
        assert_api_error(response, status, code);
        let _ = label;
    }
    let ghost = format!("/api/v1/stores/{}", Uuid::now_v7());
    assert_api_error(
        raw(&w, Method::GET, &ghost, None, "").await,
        StatusCode::NOT_FOUND,
        "",
    );
}

// === The real server wiring

#[tokio::test]
async fn health_still_works_next_to_the_json_fallbacks() {
    /*
     * main.rs merges the health router with the API router; the API's
     * fallbacks must not swallow /health or panic on merge.
     */
    let w = bare_world().await;
    let merged = atlas_backend::app().merge(w.app.clone());
    let (status, _, bytes) = send_raw(&merged, Method::GET, "/health", None, None, "").await;
    assert_eq!(
        (status, bytes.as_slice()),
        (StatusCode::OK, b"ok".as_slice())
    );
    assert_api_error(
        send_raw(&merged, Method::GET, "/missing", None, None, "").await,
        StatusCode::NOT_FOUND,
        "",
    );
}
