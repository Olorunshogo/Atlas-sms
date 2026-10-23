//! The central inventory transaction service, exercised through the endpoints that use it.

mod common;

use axum::http::StatusCode;
use common::*;
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;

fn qty(value: &str) -> Decimal {
    return value.parse().unwrap();
}

#[sqlx::test]
async fn concurrent_transfers_cannot_oversell_the_sending_store(pool: PgPool) {
    let router = build_router(pool.clone());
    let (token, tenant_id) = signup_owner(&router, &pool, "inventory-owner@example.com").await;
    let lagos = create_store(&router, &token, "lagos").await;
    let abuja = create_store(&router, &token, "abuja").await;
    let unit = seed_unit(&pool, tenant_id, "Bag", "bag").await;
    let rice = create_product(&router, &token, unit, "Rice 50kg", "RICE-50").await;
    stock_up(&router, &token, lagos, rice, "100").await;

    let transfer = json!({
        "from_store_id": lagos,
        "to_store_id": abuja,
        "product_id": rice,
        "stock_item_id": null,
        "quantity": "60",
        "reason": "Rebalance",
    });
    let caller = as_user(&token);
    let (first, second) = tokio::join!(
        post(&router, "/api/v1/stock/transfers", &caller, transfer.clone()),
        post(&router, "/api/v1/stock/transfers", &caller, transfer.clone()),
    );

    let statuses = [first.0, second.0];
    assert!(statuses.contains(&StatusCode::OK), "{:?} {:?}", first, second);
    assert!(statuses.contains(&StatusCode::CONFLICT), "{:?} {:?}", first, second);
    let rejected = if first.0 == StatusCode::CONFLICT { first.1 } else { second.1 };
    assert_eq!(rejected["code"], "PRODUCT_OUT_OF_STOCK");

    assert_eq!(qty(&on_hand(&router, &token, lagos, rice).await), qty("40"));
    assert_eq!(qty(&on_hand(&router, &token, abuja, rice).await), qty("60"));
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
