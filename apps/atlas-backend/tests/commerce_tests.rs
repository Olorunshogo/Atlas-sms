//! HTTP-level coverage for the commerce APIs added after the initial auth pass: customers, purchasing (suppliers, purchase orders, goods received notes), and payments/refunds.
//! Same `#[sqlx::test]` + real-router pattern as `auth_flow_tests.rs` - see that file for the `DATABASE_URL` requirement and why `ConnectInfo` is injected manually.

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

// === Harness (see auth_flow_tests.rs for the fuller version this is trimmed from)

fn build_router(pool: PgPool) -> Router {
    let state = AppState {
        pool,
        email: Arc::new(LogEmailProvider),
        jwt_secret: Arc::new(b"test-only-secret-not-for-production".to_vec()),
        rate_limiter: RateLimiter::with_limits(1000, std::time::Duration::from_secs(60)),
    };
    return api_router(state);
}

fn request_with_connect_info(builder: axum::http::request::Builder, body: Value) -> Request<Body> {
    let mut request = builder
        .header("content-type", "application/json")
        .header("x-client-type", "app")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));
    return request;
}

async fn send(router: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    return (status, json);
}

async fn post(router: &Router, path: &str, token: Option<&str>, body: Value) -> (StatusCode, Value) {
    let mut builder = Request::builder().method("POST").uri(path);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {}", token));
    }
    return send(router, request_with_connect_info(builder, body)).await;
}

async fn get(router: &Router, path: &str, token: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("GET")
        .uri(path)
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let mut request = request;
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));
    return send(router, request).await;
}

async fn patch(router: &Router, path: &str, token: &str, body: Value) -> (StatusCode, Value) {
    let builder = Request::builder().method("PATCH").uri(path).header("authorization", format!("Bearer {}", token));
    return send(router, request_with_connect_info(builder, body)).await;
}

/**
 * Signs up a fresh owner (their own brand-new tenant), then verifies them by updating `is_email_verified` and their pending grant directly via SQL rather than going through the real OTP endpoint - these tests care about the commerce APIs, not the auth flow itself, which is already covered in `auth_flow_tests.rs`.
 * Returns (access_token, tenant_id).
 */
async fn signup_verify_and_login(router: &Router, pool: &PgPool, email: &str) -> (String, String) {
    let (status, _) = post(
        router,
        "/api/v1/auth/signup",
        None,
        json!({
            "full_name": "Test Owner",
            "email": email,
            "phone": "+2340000000000",
            "password": "a-fairly-strong-password",
            "tenant_name": "Test Tenant",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // No capturing email provider here (see doc comment) - read the OTP
    // hash back out is pointless since it's Argon2'd; instead, mark the
    // user verified and activate the pending owner grant directly, the
    // same effect `verify-email` has, without needing the raw code.
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

    let (status, body) =
        post(router, "/api/v1/auth/login", None, json!({ "email": email, "password": "a-fairly-strong-password" }))
            .await;
    assert_eq!(status, StatusCode::OK);
    let token = body["data"]["access_token"].as_str().unwrap().to_string();
    let tenant_id = body["data"]["tenant_id"].as_str().unwrap().to_string();
    return (token, tenant_id);
}

async fn create_store(router: &Router, token: &str) -> String {
    let (status, body) =
        post(router, "/api/v1/stores", Some(token), json!({ "name": "Main Branch", "slug": "main" })).await;
    assert_eq!(status, StatusCode::OK);
    return body["data"]["id"].as_str().unwrap().to_string();
}

/// No API creates a unit of measure (a real, pre-existing gap - see `SystemDesignDocument.md`'s traceability table) - inserted directly, the same way `roles_permission_tests.rs` seeds fixtures no endpoint covers.
async fn seed_unit(pool: &PgPool, tenant_id: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO atlas.units_of_measure (id, tenant_id, name, abbreviation) VALUES ($1, $2, 'Each', 'ea')")
        .bind(id)
        .bind(Uuid::parse_str(tenant_id).unwrap())
        .execute(pool)
        .await
        .unwrap();
    return id;
}

async fn create_product(router: &Router, token: &str, base_unit_id: Uuid) -> String {
    let (status, product_type_body) = post(
        router,
        "/api/v1/product-types",
        Some(token),
        json!({ "name": "General" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let product_type_id = product_type_body["data"]["id"].as_str().unwrap().to_string();

    let (status, body) = post(
        router,
        "/api/v1/products",
        Some(token),
        json!({
            "product_type_id": product_type_id,
            "base_unit_id": base_unit_id.to_string(),
            "name": "Bottled Water",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    return body["data"]["id"].as_str().unwrap().to_string();
}

// === Customers

#[sqlx::test]
async fn registering_and_listing_a_customer_works(pool: PgPool) {
    let router = build_router(pool.clone());
    let (token, _) = signup_verify_and_login(&router, &pool, "customer-owner@example.com").await;

    let (status, body) = post(
        &router,
        "/api/v1/customers",
        Some(&token),
        json!({ "full_name": "Ada Lovelace", "phone": "+2348000000000", "email": null, "customer_group_id": null }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let customer_id = body["data"]["id"].as_str().unwrap().to_string();

    let (status, list_body) = get(&router, "/api/v1/customers", &token).await;
    assert_eq!(status, StatusCode::OK);
    let customers = list_body["data"]["items"].as_array().unwrap();
    assert!(customers.iter().any(|c| c["id"] == customer_id));

    // Partial update - only phone changes, full_name is left alone.
    let (status, updated_body) = patch(
        &router,
        &format!("/api/v1/customers/{}", customer_id),
        &token,
        json!({ "full_name": null, "phone": "+2349000000000", "email": null, "customer_group_id": null }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated_body["data"]["full_name"], "Ada Lovelace");
    assert_eq!(updated_body["data"]["phone"], "+2349000000000");
}

// === Purchasing

#[sqlx::test]
async fn a_purchase_order_moves_from_draft_to_partially_received_to_received(pool: PgPool) {
    let router = build_router(pool.clone());
    let (token, tenant_id) = signup_verify_and_login(&router, &pool, "purchasing-owner@example.com").await;
    let store_id = create_store(&router, &token).await;
    let unit_id = seed_unit(&pool, &tenant_id).await;
    let product_id = create_product(&router, &token, unit_id).await;

    let (status, supplier_body) = post(
        &router,
        "/api/v1/suppliers",
        Some(&token),
        json!({ "name": "Acme Distributors", "phone": null, "email": null }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let supplier_id = supplier_body["data"]["id"].as_str().unwrap().to_string();

    let (status, updated_supplier_body) = patch(
        &router,
        &format!("/api/v1/suppliers/{}", supplier_id),
        &token,
        json!({ "name": null, "phone": "+2348011111111", "email": null }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated_supplier_body["data"]["phone"], "+2348011111111");

    let (status, po_body) = post(
        &router,
        "/api/v1/purchase-orders",
        Some(&token),
        json!({
            "store_id": store_id,
            "supplier_id": supplier_id,
            "lines": [{ "product_id": product_id, "quantity_ordered": "10", "unit_cost_amount_minor": 500 }],
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let purchase_order_id = po_body["data"]["id"].as_str().unwrap().to_string();
    assert_eq!(po_body["data"]["status"], "draft");

    let (status, lines_body) =
        get(&router, &format!("/api/v1/purchase-orders/{}/lines", purchase_order_id), &token).await;
    assert_eq!(status, StatusCode::OK);
    let lines = lines_body["data"].as_array().unwrap();
    assert_eq!(lines.len(), 1);
    let line_id = lines[0]["id"].as_str().unwrap().to_string();

    // Receiving more than was ordered on this line is rejected...
    let (status, _) = post(
        &router,
        &format!("/api/v1/purchase-orders/{}/receive", purchase_order_id),
        Some(&token),
        json!({ "lines": [{ "purchase_order_line_id": line_id, "quantity_received": "11" }] }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // ...a short shipment is accepted and moves the order to partially_received.
    let (status, receive_body) = post(
        &router,
        &format!("/api/v1/purchase-orders/{}/receive", purchase_order_id),
        Some(&token),
        json!({ "lines": [{ "purchase_order_line_id": line_id, "quantity_received": "6" }] }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(receive_body["data"]["purchase_order"]["status"], "partially_received");

    // Real stock now on hand from the partial receipt.
    let (status, stock_body) = get(&router, &format!("/api/v1/stock/on-hand?store_id={}", store_id), &token).await;
    assert_eq!(status, StatusCode::OK);
    let levels = stock_body["data"].as_array().unwrap();
    assert_eq!(levels[0]["on_hand"], "6");

    // Receiving the rest completes the order.
    let (status, receive_body) = post(
        &router,
        &format!("/api/v1/purchase-orders/{}/receive", purchase_order_id),
        Some(&token),
        json!({ "lines": [{ "purchase_order_line_id": line_id, "quantity_received": "4" }] }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(receive_body["data"]["purchase_order"]["status"], "received");

    // The supplier's balance reflects both receipts: 10 units * 500 minor = 5000.
    let (status, suppliers_body) = get(&router, "/api/v1/suppliers", &token).await;
    assert_eq!(status, StatusCode::OK);
    let supplier = suppliers_body["data"].as_array().unwrap().iter().find(|s| s["id"] == supplier_id).unwrap();
    assert_eq!(supplier["balance_amount_minor"], 5000);
}

// === Payments

#[sqlx::test]
async fn recording_a_payment_and_running_it_through_the_refund_flow_works(pool: PgPool) {
    let router = build_router(pool.clone());
    let (token, tenant_id) = signup_verify_and_login(&router, &pool, "payments-owner@example.com").await;
    let store_id = create_store(&router, &token).await;
    let unit_id = seed_unit(&pool, &tenant_id).await;
    let product_id = create_product(&router, &token, unit_id).await;

    let (status, shift_body) =
        post(&router, "/api/v1/shifts", Some(&token), json!({ "store_id": store_id, "opening_cash_amount_minor": 0 })).await;
    assert_eq!(status, StatusCode::OK);
    let shift_id = shift_body["data"]["id"].as_str().unwrap().to_string();

    let sale_id = Uuid::now_v7().to_string();
    let (status, sale_body) = post(
        &router,
        "/api/v1/sales",
        Some(&token),
        json!({
            "id": sale_id,
            "store_id": store_id,
            "shift_id": shift_id,
            "customer_id": null,
            "device_id": "test-device",
            "discount_amount_minor": 0,
            "tax_amount_minor": 0,
            "lines": [{ "product_id": product_id, "stock_item_id": null, "unit_id": unit_id.to_string(), "quantity": "1", "unit_price_amount_minor": 1000 }],
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sale_body["data"]["id"], sale_id);

    let (status, payment_body) = post(
        &router,
        "/api/v1/payments",
        Some(&token),
        json!({ "sale_id": sale_id, "method": "cash", "amount_minor": 1000, "currency": "NGN" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payment_id = payment_body["data"]["id"].as_str().unwrap().to_string();
    assert_eq!(payment_body["data"]["status"], "succeeded");

    let (status, refund_request_body) = post(
        &router,
        &format!("/api/v1/payments/{}/refund", payment_id),
        Some(&token),
        json!({ "amount_minor": 1000 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let refund_id = refund_request_body["data"]["refund"]["id"].as_str().unwrap().to_string();
    let confirmation_token = refund_request_body["data"]["confirmation_token"].as_str().unwrap().to_string();
    assert_eq!(refund_request_body["data"]["refund"]["status"], "awaiting_confirmation");

    // The wrong token is rejected...
    let (status, _) = post(
        &router,
        &format!("/api/v1/payments/refunds/{}/confirm", refund_id),
        Some(&token),
        json!({ "confirmation_token": "not-the-right-token" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // ...the right one executes it.
    let (status, confirm_body) = post(
        &router,
        &format!("/api/v1/payments/refunds/{}/confirm", refund_id),
        Some(&token),
        json!({ "confirmation_token": confirmation_token }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(confirm_body["data"]["status"], "executed");

    // Already executed - confirming again (even with the right token) fails.
    let (status, _) = post(
        &router,
        &format!("/api/v1/payments/refunds/{}/confirm", refund_id),
        Some(&token),
        json!({ "confirmation_token": confirmation_token }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
