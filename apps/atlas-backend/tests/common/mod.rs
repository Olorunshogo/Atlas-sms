//! Shared harness for the sales, payments, receipts, returns and customer integration tests.
//! Same `#[sqlx::test]` + real-router pattern as `commerce_tests.rs`; needs a plain `DATABASE_URL` pointing at a Postgres that can create databases.

#![allow(dead_code)]

use std::net::SocketAddr;
use std::sync::Arc;

use atlas_backend::{api_router, AppState, LogEmailProvider, RateLimiter};
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

pub const JWT_SECRET: &[u8] = b"test-only-secret-not-for-production";

// === Router and requests

pub fn build_router(pool: PgPool) -> Router {
    let state = AppState {
        pool,
        email: Arc::new(LogEmailProvider),
        jwt_secret: Arc::new(JWT_SECRET.to_vec()),
        rate_limiter: RateLimiter::with_limits(1000, std::time::Duration::from_secs(60)),
        integrations: atlas_backend::integrations::Integrations::mock(b"test-webhook-secret"),
    };
    return api_router(state);
}

pub struct Caller<'a> {
    pub token: &'a str,
    pub store_id: Option<&'a str>,
}

pub fn as_user(token: &str) -> Caller<'_> {
    return Caller { token, store_id: None };
}

pub fn at_store<'a>(token: &'a str, store_id: &'a str) -> Caller<'a> {
    return Caller { token, store_id: Some(store_id) };
}

pub async fn call(router: &Router, method: &str, path: &str, caller: Option<&Caller<'_>>, body: Option<Value>) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .header("x-client-type", "app");
    if let Some(caller) = caller {
        builder = builder.header("authorization", format!("Bearer {}", caller.token));
        if let Some(store_id) = caller.store_id {
            builder = builder.header("x-store-id", store_id);
        }
    }
    let body = match body {
        Some(value) => Body::from(serde_json::to_vec(&value).unwrap()),
        None => Body::empty(),
    };
    let mut request = builder.body(body).unwrap();
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));

    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).to_string()))
    };
    return (status, json);
}

/// Like `call`, but the caller controls the exact body bytes and content type, to exercise input that `serde_json::Value` cannot express. Returns the status, the response content type and the raw body.
pub async fn call_raw(
    router: &Router,
    method: &str,
    path: &str,
    token: Option<&str>,
    content_type: Option<&str>,
    body: &str,
) -> (StatusCode, Option<String>, Vec<u8>) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {}", token));
    }
    if let Some(content_type) = content_type {
        builder = builder.header("content-type", content_type);
    }
    let mut request = builder.body(Body::from(body.to_string())).unwrap();
    request.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let content_type = response.headers().get("content-type").map(|v| v.to_str().unwrap().to_string());
    let bytes = response.into_body().collect().await.unwrap().to_bytes().to_vec();
    return (status, content_type, bytes);
}

pub async fn post(router: &Router, path: &str, caller: &Caller<'_>, body: Value) -> (StatusCode, Value) {
    return call(router, "POST", path, Some(caller), Some(body)).await;
}

pub async fn get(router: &Router, path: &str, caller: &Caller<'_>) -> (StatusCode, Value) {
    return call(router, "GET", path, Some(caller), None).await;
}

pub async fn patch(router: &Router, path: &str, caller: &Caller<'_>, body: Value) -> (StatusCode, Value) {
    return call(router, "PATCH", path, Some(caller), Some(body)).await;
}

pub async fn delete(router: &Router, path: &str, caller: &Caller<'_>) -> (StatusCode, Value) {
    return call(router, "DELETE", path, Some(caller), None).await;
}

pub fn id_of(body: &Value) -> String {
    return body["data"]["id"].as_str().unwrap_or_else(|| panic!("no data.id in {}", body)).to_string();
}

// === Identity

/**
 * Signs up a fresh owner with their own tenant and activates them directly in SQL, the same shortcut `commerce_tests.rs` takes.
 * Returns (access_token, tenant_id).
 */
pub async fn signup_owner(router: &Router, pool: &PgPool, email: &str) -> (String, Uuid) {
    let (status, _) = call(
        router,
        "POST",
        "/api/v1/auth/signup",
        None,
        Some(json!({
            "full_name": "Test Owner",
            "email": email,
            "phone": "+2340000000000",
            "password": "a-fairly-strong-password",
            "tenant_name": "Test Tenant",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    sqlx::query("UPDATE atlas.users SET is_email_verified = true WHERE email = $1")
        .bind(email)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE atlas.role_grants SET status = 'active'
         WHERE user_id = (SELECT id FROM atlas.users WHERE email = $1) AND status = 'pending'",
    )
    .bind(email)
    .execute(pool)
    .await
    .unwrap();

    let (status, body) = call(
        router,
        "POST",
        "/api/v1/auth/login",
        None,
        Some(json!({ "email": email, "password": "a-fairly-strong-password" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let token = body["data"]["access_token"].as_str().unwrap().to_string();
    let tenant_id = Uuid::parse_str(body["data"]["tenant_id"].as_str().unwrap()).unwrap();
    return (token, tenant_id);
}

#[derive(serde::Serialize)]
struct TestClaims {
    sub: Uuid,
    tenant_id: Uuid,
    exp: usize,
}

pub fn issue_token(user_id: Uuid, tenant_id: Uuid) -> String {
    let claims = TestClaims {
        sub: user_id,
        tenant_id,
        exp: (chrono::Utc::now() + chrono::Duration::minutes(15)).timestamp() as usize,
    };
    return jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(JWT_SECRET),
    )
    .unwrap();
}

/// A staff member holding one seeded system role (`cashier`, `store_manager`, ...) scoped to a single store. Returns (user_id, access_token).
pub async fn seed_staff(pool: &PgPool, tenant_id: Uuid, store_id: Uuid, role_name: &str) -> (Uuid, String) {
    let user_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO atlas.users (id, full_name, email, password_hash, is_email_verified)
         VALUES ($1, $2, $3, 'not-a-real-hash', true)",
    )
    .bind(user_id)
    .bind(format!("Staff {}", role_name))
    .bind(format!("{}-{}@example.com", role_name, user_id))
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO atlas.role_grants (id, user_id, role_id, scope, tenant_id, store_id, status, granted_by_user_id)
         SELECT $1, $2, r.id, 'store', $3, $4, 'active', $2
         FROM atlas.roles r WHERE r.is_system AND r.name = $5",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(tenant_id)
    .bind(store_id)
    .bind(role_name)
    .execute(pool)
    .await
    .unwrap();

    return (user_id, issue_token(user_id, tenant_id));
}

// === Catalogue fixtures

pub async fn create_store(router: &Router, token: &str, slug: &str) -> Uuid {
    let (status, body) = post(router, "/api/v1/stores", &as_user(token), json!({ "name": format!("Store {}", slug), "slug": slug })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    return Uuid::parse_str(&id_of(&body)).unwrap();
}

pub async fn seed_unit(pool: &PgPool, tenant_id: Uuid, name: &str, abbreviation: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO atlas.units_of_measure (id, tenant_id, name, abbreviation) VALUES ($1, $2, $3, $4)")
        .bind(id)
        .bind(tenant_id)
        .bind(name)
        .bind(abbreviation)
        .execute(pool)
        .await
        .unwrap();
    return id;
}

pub async fn create_product(router: &Router, token: &str, base_unit_id: Uuid, name: &str, sku: &str) -> Uuid {
    let (status, product_type_body) = post(router, "/api/v1/product-types", &as_user(token), json!({ "name": format!("Type {}", sku) })).await;
    assert_eq!(status, StatusCode::OK, "{}", product_type_body);
    let product_type_id = id_of(&product_type_body);

    let (status, body) = post(
        router,
        "/api/v1/products",
        &as_user(token),
        json!({
            "product_type_id": product_type_id,
            "base_unit_id": base_unit_id.to_string(),
            "name": name,
            "sku": sku,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    return Uuid::parse_str(&id_of(&body)).unwrap();
}

// === Stock fixtures

/// Puts `quantity` base units on hand through the real adjustment endpoint (so it goes through the inventory service).
pub async fn stock_up(router: &Router, token: &str, store_id: Uuid, product_id: Uuid, quantity: &str) {
    let (status, body) = post(
        router,
        "/api/v1/stock/adjustments",
        &as_user(token),
        json!({
            "store_id": store_id,
            "product_id": product_id,
            "stock_item_id": null,
            "quantity": quantity,
            "reason": "Opening count",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
}

pub async fn on_hand(router: &Router, token: &str, store_id: Uuid, product_id: Uuid) -> String {
    let (status, body) = get(router, &format!("/api/v1/stock/on-hand?store_id={}", store_id), &as_user(token)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    return body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|level| level["product_id"] == product_id.to_string())
        .map(|level| level["on_hand"].as_str().unwrap().to_string())
        .unwrap_or_else(|| "0".to_string());
}

// === Selling fixtures

pub async fn list_at(router: &Router, token: &str, product_id: Uuid, store_id: Uuid) {
    let (status, body) = call(
        router,
        "PUT",
        &format!("/api/v1/products/{}/listings/{}", product_id, store_id),
        Some(&as_user(token)),
        Some(json!({ "cost_amount_minor": null, "low_stock_threshold": null, "is_active": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
}

/// A price rule per base unit; `store_id: None` applies at every store.
pub async fn price_rule(router: &Router, token: &str, product_id: Uuid, store_id: Option<Uuid>, amount_minor: i64) -> Uuid {
    let (status, body) = post(
        router,
        &format!("/api/v1/products/{}/price-rules", product_id),
        &as_user(token),
        json!({ "store_id": store_id, "unit_price_amount_minor": amount_minor }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    return Uuid::parse_str(&id_of(&body)).unwrap();
}

pub async fn sale_unit(router: &Router, token: &str, product_id: Uuid, unit_id: Uuid, factor: &str) {
    let (status, body) = post(
        router,
        &format!("/api/v1/products/{}/sale-units", product_id),
        &as_user(token),
        json!({ "unit_id": unit_id, "factor_to_base": factor }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
}

pub async fn barcode(router: &Router, token: &str, product_id: Uuid, unit_id: Option<Uuid>, code: &str) {
    let (status, body) = post(
        router,
        &format!("/api/v1/products/{}/barcodes", product_id),
        &as_user(token),
        json!({ "barcode": code, "unit_id": unit_id }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
}

pub async fn tax_rate(router: &Router, token: &str, name: &str, basis_points: i32) -> Uuid {
    let (status, body) = post(router, "/api/v1/tax-rates", &as_user(token), json!({ "name": name, "rate_basis_points": basis_points })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    return Uuid::parse_str(&id_of(&body)).unwrap();
}

pub async fn store_settings(router: &Router, token: &str, store_id: Uuid, settings: Value) {
    let (status, body) = patch(router, &format!("/api/v1/stores/{}/settings", store_id), &as_user(token), settings).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
}

/// Sets an approval policy; `limit` is the one limit field, e.g. `json!({ "limit_quantity": "10" })`. `store_id: None` is business-wide.
pub async fn approval_policy(router: &Router, token: &str, store_id: Option<Uuid>, operation: &str, limit: Value) {
    let mut body = json!({ "store_id": store_id, "operation": operation });
    body.as_object_mut().unwrap().extend(limit.as_object().unwrap().clone());
    let (status, body) = call(router, "PUT", "/api/v1/approval-policies", Some(&as_user(token)), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
}

/// A sellable product: created, listed at `store_id`, priced per base unit, with `stock` on hand.
#[allow(clippy::too_many_arguments)]
pub async fn sellable(
    router: &Router,
    token: &str,
    store_id: Uuid,
    unit_id: Uuid,
    name: &str,
    sku: &str,
    price_minor: i64,
    stock: &str,
) -> Uuid {
    let product_id = create_product(router, token, unit_id, name, sku).await;
    list_at(router, token, product_id, store_id).await;
    price_rule(router, token, product_id, None, price_minor).await;
    if stock != "0" {
        stock_up(router, token, store_id, product_id, stock).await;
    }
    return product_id;
}

// === Sale fixtures

pub async fn open_shift(router: &Router, token: &str, store_id: Uuid) -> Uuid {
    let (status, body) = post(router, "/api/v1/shifts", &as_user(token), json!({ "store_id": store_id, "opening_cash_amount_minor": 0 })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    return Uuid::parse_str(&id_of(&body)).unwrap();
}

/// The id of one of the tenant's payment methods by code (`cash`, `card`, `transfer`, `store_credit`).
pub async fn method_id(router: &Router, token: &str, code: &str) -> Uuid {
    let (status, body) = get(router, "/api/v1/payment-methods", &as_user(token)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let id = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["code"] == code)
        .unwrap_or_else(|| panic!("no payment method {}", code))["id"]
        .as_str()
        .unwrap()
        .to_string();
    return Uuid::parse_str(&id).unwrap();
}

pub fn cart_line(product_id: Uuid, unit_id: Uuid, quantity: &str) -> Value {
    return json!({ "product_id": product_id, "unit_id": unit_id, "quantity": quantity });
}

pub fn pay(method_id: Uuid, amount_minor: i64) -> Value {
    return json!({ "id": Uuid::now_v7(), "payment_method_id": method_id, "amount_minor": amount_minor });
}

/// A sale request body with a fresh client id; tweak the returned JSON for anything unusual.
pub fn sale_request(store_id: Uuid, shift_id: Uuid, lines: Vec<Value>, payments: Vec<Value>) -> Value {
    return json!({
        "id": Uuid::now_v7(),
        "store_id": store_id,
        "shift_id": shift_id,
        "customer_id": null,
        "device_id": "till-1",
        "lines": lines,
        "payments": payments,
    });
}
