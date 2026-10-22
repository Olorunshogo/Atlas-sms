//! Store-scoped grants against tenant-level resources: a cashier's grant only reaches a customer endpoint when the request names the cashier's own store.

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test]
async fn store_scoped_cashier_registers_customer_only_at_own_store(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner_token, tenant_id) = signup_owner(&router, &pool, "scope-owner@example.com").await;
    let store_id = create_store(&router, &owner_token, "ikeja").await;
    let other_store_id = create_store(&router, &owner_token, "lekki").await;
    let (_, cashier_token) = seed_staff(&pool, tenant_id, store_id, "cashier").await;

    let body = json!({ "full_name": "Walk In", "phone": null, "email": null, "customer_group_id": null });

    let (status, _) = post(&router, "/api/v1/customers", &as_user(&cashier_token), body.clone()).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let other = other_store_id.to_string();
    let (status, _) = post(&router, "/api/v1/customers", &at_store(&cashier_token, &other), body.clone()).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let own = store_id.to_string();
    let (status, created) = post(&router, "/api/v1/customers", &at_store(&cashier_token, &own), body).await;
    assert_eq!(status, StatusCode::OK, "{}", created);
}

#[sqlx::test]
async fn closing_a_shift_needs_sale_create_at_its_store(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner_token, tenant_id) = signup_owner(&router, &pool, "shift-owner@example.com").await;
    let store_id = create_store(&router, &owner_token, "ikeja").await;
    let other_store_id = create_store(&router, &owner_token, "lekki").await;
    let (_, outsider_token) = seed_staff(&pool, tenant_id, other_store_id, "cashier").await;

    let (status, shift) = post(&router, "/api/v1/shifts", &as_user(&owner_token), json!({ "store_id": store_id })).await;
    assert_eq!(status, StatusCode::OK, "{}", shift);
    let shift_id = id_of(&shift);

    let (status, _) = post(
        &router,
        &format!("/api/v1/shifts/{}/close", shift_id),
        &as_user(&outsider_token),
        json!({ "closing_cash_amount_minor": 0 }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
