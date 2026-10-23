//! The central inventory transaction service, exercised through the endpoints that use it.

mod common;

use axum::http::StatusCode;
use axum::Router;
use common::*;
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn qty(value: &str) -> Decimal {
    return value.parse().unwrap();
}

async fn transfer_id(router: &Router, token: &str, from: Uuid, to: Uuid, product: Uuid, quantity: &str) -> Uuid {
    let (status, body) = post(
        router,
        "/api/v1/stock/transfers",
        &as_user(token),
        json!({
            "from_store_id": from,
            "to_store_id": to,
            "product_id": product,
            "stock_item_id": null,
            "quantity": quantity,
            "reason": "Rebalance",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{:?}", body);
    return Uuid::parse_str(body["data"]["id"].as_str().unwrap()).unwrap();
}

async fn approve(router: &Router, token: &str, id: Uuid) -> (StatusCode, serde_json::Value) {
    return post(
        router,
        &format!("/api/v1/stock/transfers/{id}/approve"),
        &as_user(token),
        json!({}),
    )
    .await;
}

#[sqlx::test]
async fn a_requested_transfer_moves_stock_only_at_dispatch_and_receive(pool: PgPool) {
    let router = build_router(pool.clone());
    let (token, tenant_id) = signup_owner(&router, &pool, "inventory-owner@example.com").await;
    let lagos = create_store(&router, &token, "lagos").await;
    let abuja = create_store(&router, &token, "abuja").await;
    let unit = seed_unit(&pool, tenant_id, "Bag", "bag").await;
    let rice = create_product(&router, &token, unit, "Rice 50kg", "RICE-50").await;
    stock_up(&router, &token, lagos, rice, "100").await;

    let id = transfer_id(&router, &token, lagos, abuja, rice, "60").await;
    assert_eq!(qty(&on_hand(&router, &token, lagos, rice).await), qty("100"));

    let (status, _) = approve(&router, &token, id).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(qty(&on_hand(&router, &token, lagos, rice).await), qty("100"));

    let (status, _) = post(&router, &format!("/api/v1/stock/transfers/{id}/dispatch"), &as_user(&token), json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(qty(&on_hand(&router, &token, lagos, rice).await), qty("40"));
    assert_eq!(qty(&on_hand(&router, &token, abuja, rice).await), qty("0"));

    let (status, _) = post(&router, &format!("/api/v1/stock/transfers/{id}/receive"), &as_user(&token), json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(qty(&on_hand(&router, &token, abuja, rice).await), qty("60"));
}

#[sqlx::test]
async fn concurrent_dispatches_cannot_oversell_the_sending_store(pool: PgPool) {
    let router = build_router(pool.clone());
    let (token, tenant_id) = signup_owner(&router, &pool, "inventory-owner@example.com").await;
    let lagos = create_store(&router, &token, "lagos").await;
    let abuja = create_store(&router, &token, "abuja").await;
    let unit = seed_unit(&pool, tenant_id, "Bag", "bag").await;
    let rice = create_product(&router, &token, unit, "Rice 50kg", "RICE-50").await;
    stock_up(&router, &token, lagos, rice, "100").await;

    // Both requests pass the honest, non-reserving check at request time -
    // the real protection is the atomic claim in `dispatch_transfer`.
    let first_id = transfer_id(&router, &token, lagos, abuja, rice, "60").await;
    let second_id = transfer_id(&router, &token, lagos, abuja, rice, "60").await;
    approve(&router, &token, first_id).await;
    approve(&router, &token, second_id).await;

    let caller = as_user(&token);
    let first_path = format!("/api/v1/stock/transfers/{first_id}/dispatch");
    let second_path = format!("/api/v1/stock/transfers/{second_id}/dispatch");
    let (first, second) = tokio::join!(
        post(&router, &first_path, &caller, json!({})),
        post(&router, &second_path, &caller, json!({})),
    );

    let statuses = [first.0, second.0];
    assert!(statuses.contains(&StatusCode::OK), "{:?} {:?}", first, second);
    assert!(statuses.contains(&StatusCode::CONFLICT), "{:?} {:?}", first, second);
    let rejected = if first.0 == StatusCode::CONFLICT { first.1 } else { second.1 };
    assert_eq!(rejected["code"], "PRODUCT_OUT_OF_STOCK");

    assert_eq!(qty(&on_hand(&router, &token, lagos, rice).await), qty("40"));
}

#[sqlx::test]
async fn a_rejected_transfer_moves_no_stock(pool: PgPool) {
    let router = build_router(pool.clone());
    let (token, tenant_id) = signup_owner(&router, &pool, "reject-owner@example.com").await;
    let lagos = create_store(&router, &token, "lagos").await;
    let abuja = create_store(&router, &token, "abuja").await;
    let unit = seed_unit(&pool, tenant_id, "Bag", "bag").await;
    let rice = create_product(&router, &token, unit, "Rice 50kg", "RICE-50").await;
    stock_up(&router, &token, lagos, rice, "100").await;

    let id = transfer_id(&router, &token, lagos, abuja, rice, "60").await;
    let (status, _) = post(&router, &format!("/api/v1/stock/transfers/{id}/reject"), &as_user(&token), json!({})).await;
    assert_eq!(status, StatusCode::OK);

    // An already-rejected transfer cannot then be approved.
    let (status, body) = approve(&router, &token, id).await;
    assert_eq!(status, StatusCode::CONFLICT, "{:?}", body);
    assert_eq!(body["code"], "TRANSFER_NOT_REQUESTED");
    assert_eq!(qty(&on_hand(&router, &token, lagos, rice).await), qty("100"));
}

#[sqlx::test]
async fn an_approved_transfer_can_be_cancelled_but_a_dispatched_one_cannot(pool: PgPool) {
    let router = build_router(pool.clone());
    let (token, tenant_id) = signup_owner(&router, &pool, "cancel-owner@example.com").await;
    let lagos = create_store(&router, &token, "lagos").await;
    let abuja = create_store(&router, &token, "abuja").await;
    let unit = seed_unit(&pool, tenant_id, "Bag", "bag").await;
    let rice = create_product(&router, &token, unit, "Rice 50kg", "RICE-50").await;
    stock_up(&router, &token, lagos, rice, "100").await;

    let id = transfer_id(&router, &token, lagos, abuja, rice, "60").await;
    approve(&router, &token, id).await;
    let (status, _) = post(&router, &format!("/api/v1/stock/transfers/{id}/cancel"), &as_user(&token), json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(qty(&on_hand(&router, &token, lagos, rice).await), qty("100"));

    let other_id = transfer_id(&router, &token, lagos, abuja, rice, "20").await;
    approve(&router, &token, other_id).await;
    post(&router, &format!("/api/v1/stock/transfers/{other_id}/dispatch"), &as_user(&token), json!({})).await;
    let (status, body) = post(&router, &format!("/api/v1/stock/transfers/{other_id}/cancel"), &as_user(&token), json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{:?}", body);
    assert_eq!(body["code"], "TRANSFER_NOT_CANCELLABLE");
}

#[sqlx::test]
async fn adjustments_are_never_blocked_by_the_negative_stock_check(pool: PgPool) {
    let router = build_router(pool.clone());
    let (token, tenant_id) = signup_owner(&router, &pool, "adjust-owner@example.com").await;
    let store = create_store(&router, &token, "lagos").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let product = create_product(&router, &token, unit, "Soap", "SOAP").await;

    stock_up(&router, &token, store, product, "-3").await;
    assert_eq!(qty(&on_hand(&router, &token, store, product).await), qty("-3"));
    stock_up(&router, &token, store, product, "10").await;
    assert_eq!(qty(&on_hand(&router, &token, store, product).await), qty("7"));
}

#[sqlx::test]
async fn a_zero_quantity_posting_is_rejected(pool: PgPool) {
    let router = build_router(pool.clone());
    let (token, tenant_id) = signup_owner(&router, &pool, "zero-owner@example.com").await;
    let store = create_store(&router, &token, "lagos").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let product = create_product(&router, &token, unit, "Soap", "SOAP").await;

    let (status, _) = post(
        &router,
        "/api/v1/stock/adjustments",
        &as_user(&token),
        json!({ "store_id": store, "product_id": product, "stock_item_id": null, "quantity": "0", "reason": "noop" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
