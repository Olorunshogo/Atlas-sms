//! Every failure is `{ message, success: false, code? }` as JSON, including the ones the framework produces before a handler runs.

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{json, Value};
use sqlx::PgPool;

/// Asserts the response is a JSON error with the given status and (when non-empty) code, and returns the body.
fn assert_api_error(response: (StatusCode, Option<String>, Vec<u8>), status: StatusCode, code: &str) -> Value {
    let (actual, content_type, bytes) = response;
    let text = String::from_utf8_lossy(&bytes).to_string();
    assert_eq!(actual, status, "{}", text);
    assert!(content_type.as_deref().is_some_and(|v| v.starts_with("application/json")), "expected JSON, got {:?}: {}", content_type, text);
    let body: Value = serde_json::from_slice(&bytes).unwrap_or_else(|_| panic!("not JSON: {}", text));
    assert_eq!(body["success"], false, "{}", body);
    assert!(body["message"].as_str().is_some_and(|m| !m.is_empty()), "{}", body);
    if code.is_empty() {
        assert!(body["code"].is_null(), "{}", body);
    } else {
        assert_eq!(body["code"], code, "{}", body);
    }
    return body;
}

#[sqlx::test]
async fn malformed_and_wrong_shaped_bodies_are_json(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, _) = signup_owner(&router, &pool, "shape-owner@example.com").await;
    let post_stores = |content_type: Option<&'static str>, body: &'static str| {
        let (router, owner) = (router.clone(), owner.clone());
        async move { call_raw(&router, "POST", "/api/v1/stores", Some(&owner), content_type, body).await }
    };

    for broken in ["{ not json", "", "   ", "{\"name\": }", "nope"] {
        assert_api_error(post_stores(Some("application/json"), broken).await, StatusCode::BAD_REQUEST, "INVALID_REQUEST");
    }
    for wrong in [r#"{"name": "No slug"}"#, r#"{"name": 42, "slug": "x"}"#, r#"{"name": "A", "slug": null}"#] {
        assert_api_error(post_stores(Some("application/json"), wrong).await, StatusCode::UNPROCESSABLE_ENTITY, "INVALID_REQUEST");
    }
    for content_type in [None, Some("text/plain"), Some("application/x-www-form-urlencoded")] {
        assert_api_error(post_stores(content_type, r#"{"name":"A","slug":"a"}"#).await, StatusCode::UNSUPPORTED_MEDIA_TYPE, "INVALID_REQUEST");
    }
    let (_, stores) = get(&router, "/api/v1/stores", &as_user(&owner)).await;
    assert!(stores["data"].as_array().unwrap().is_empty(), "nothing was created by any of them");
}

#[sqlx::test]
async fn unknown_and_immutable_fields_name_the_field(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, _) = signup_owner(&router, &pool, "field-owner@example.com").await;
    let store = create_store(&router, &owner, "lagos").await;
    let response = call_raw(&router, "PATCH", &format!("/api/v1/stores/{}", store), Some(&owner), Some("application/json"), r#"{"currency":"USD"}"#).await;
    let body = assert_api_error(response, StatusCode::UNPROCESSABLE_ENTITY, "INVALID_REQUEST");
    assert!(body["message"].as_str().unwrap().contains("currency"), "{}", body);
}

#[sqlx::test]
async fn bad_ids_in_paths_and_queries_are_json(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, _) = signup_owner(&router, &pool, "id-owner@example.com").await;
    for (method, path) in [
        ("GET", "/api/v1/stores/not-a-uuid"),
        ("GET", "/api/v1/products/12345"),
        ("GET", "/api/v1/stock/products/abc"),
        ("GET", "/api/v1/sales/xyz"),
        ("DELETE", "/api/v1/products/abc/price-rules/def"),
    ] {
        assert_api_error(call_raw(&router, method, path, Some(&owner), Some("application/json"), "").await, StatusCode::BAD_REQUEST, "INVALID_REQUEST");
    }
    for path in [
        "/api/v1/stock/on-hand?store_id=abc",
        "/api/v1/stock/on-hand",
        "/api/v1/stock/inventory?low_stock=maybe",
        "/api/v1/stock/movements?limit=lots",
    ] {
        assert_api_error(call_raw(&router, "GET", path, Some(&owner), None, "").await, StatusCode::BAD_REQUEST, "INVALID_REQUEST");
    }
}

#[sqlx::test]
async fn unknown_routes_wrong_methods_and_missing_auth_are_json(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, _) = signup_owner(&router, &pool, "route-owner@example.com").await;
    for path in ["/api/v1/nothing-here", "/api/v1/stock/unknown", "/elsewhere"] {
        assert_api_error(call_raw(&router, "GET", path, Some(&owner), None, "").await, StatusCode::NOT_FOUND, "ROUTE_NOT_FOUND");
    }
    // GET /stock/transfers is a valid route (list transfers) since the FR-22
    // staged-transfer workflow landed - PUT still covers "wrong method" here.
    for (method, path) in [("DELETE", "/api/v1/stores"), ("PUT", "/api/v1/stock/transfers")] {
        assert_api_error(call_raw(&router, method, path, Some(&owner), None, "").await, StatusCode::METHOD_NOT_ALLOWED, "METHOD_NOT_ALLOWED");
    }
    for token in [None, Some("garbage")] {
        assert_api_error(call_raw(&router, "GET", "/api/v1/stores", token, None, "").await, StatusCode::UNAUTHORIZED, "");
    }
}

#[sqlx::test]
async fn handler_errors_keep_the_same_shape_and_the_health_route_survives(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, _) = signup_owner(&router, &pool, "handler-owner@example.com").await;
    create_store(&router, &owner, "lagos").await;
    let (status, body) = post(&router, "/api/v1/stores", &as_user(&owner), json!({ "name": "Dup", "slug": "lagos" })).await;
    assert_eq!((status, body["code"].as_str()), (StatusCode::CONFLICT, Some("ALREADY_EXISTS")));
    assert_eq!(body["success"], false);

    // main.rs merges the health router with the API router; the JSON handling must not swallow /health.
    let merged = atlas_backend::app().merge(router.clone());
    let (status, _, bytes) = call_raw(&merged, "GET", "/health", None, None, "").await;
    assert_eq!((status, bytes.as_slice()), (StatusCode::OK, b"ok".as_slice()));
    assert_api_error(call_raw(&merged, "GET", "/missing", None, None, "").await, StatusCode::NOT_FOUND, "ROUTE_NOT_FOUND");
}
