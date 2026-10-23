//! FR-26 notifications and FR-13 low-stock alerts: who receives them, duplicate control, read state, and that a failed notification never undoes the action behind it.

mod common;

use axum::http::StatusCode;
use axum::Router;
use common::*;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

async fn inbox(router: &Router, caller: &Caller<'_>) -> Vec<Value> {
    let (status, body) = get(router, "/api/v1/notifications", caller).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    return body["data"].as_array().unwrap().clone();
}

async fn adjust(router: &Router, token: &str, store_id: Uuid, product_id: Uuid, quantity: &str) {
    let (status, body) = post(
        router,
        "/api/v1/stock/adjustments",
        &as_user(token),
        json!({ "store_id": store_id, "product_id": product_id, "stock_item_id": null, "quantity": quantity, "reason": "Count" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
}

async fn set_threshold(router: &Router, token: &str, product_id: Uuid, store_id: Uuid, threshold: &str) {
    let (status, body) = call(
        router,
        "PUT",
        &format!("/api/v1/products/{}/listings/{}", product_id, store_id),
        Some(&as_user(token)),
        Some(json!({ "cost_amount_minor": null, "low_stock_threshold": threshold, "is_active": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
}

#[sqlx::test]
async fn crossing_the_reorder_level_alerts_once_until_stock_recovers(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "low-stock-owner@example.com").await;
    let store = create_store(&router, &owner, "low-stock").await;
    let unit = seed_unit(&pool, tenant_id, "Bag", "bag").await;
    let rice = create_product(&router, &owner, unit, "Rice 50kg", "RICE-LS").await;
    set_threshold(&router, &owner, rice, store, "5").await;
    let (_, manager) = seed_staff(&pool, tenant_id, store, "store_manager").await;
    let (_, cashier) = seed_staff(&pool, tenant_id, store, "cashier").await;
    let store_str = store.to_string();
    let manager_caller = at_store(&manager, &store_str);

    stock_up(&router, &owner, store, rice, "10").await;
    assert!(inbox(&router, &manager_caller).await.is_empty());

    adjust(&router, &owner, store, rice, "-6").await;
    let alerts = inbox(&router, &manager_caller).await;
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0]["kind"], "low_stock");
    assert_eq!(alerts[0]["status"], "unread");
    assert_eq!(alerts[0]["store_id"], store_str);
    assert_eq!(alerts[0]["entity_id"], rice.to_string());

    // Still low: the open alert already covers it
    adjust(&router, &owner, store, rice, "-2").await;
    assert_eq!(inbox(&router, &manager_caller).await.len(), 1);

    // Recovery resolves it, so the next drop is a new condition
    adjust(&router, &owner, store, rice, "10").await;
    adjust(&router, &owner, store, rice, "-9").await;
    assert_eq!(inbox(&router, &manager_caller).await.len(), 2);

    assert!(inbox(&router, &at_store(&cashier, &store_str)).await.is_empty());
}

#[sqlx::test]
async fn a_transfer_request_reaches_the_destination_store_only(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "transfer-notify-owner@example.com").await;
    let lagos = create_store(&router, &owner, "notify-lagos").await;
    let abuja = create_store(&router, &owner, "notify-abuja").await;
    let kano = create_store(&router, &owner, "notify-kano").await;
    let unit = seed_unit(&pool, tenant_id, "Bag", "bag").await;
    let rice = create_product(&router, &owner, unit, "Rice 50kg", "RICE-TN").await;
    stock_up(&router, &owner, lagos, rice, "50").await;
    let (_, lagos_manager) = seed_staff(&pool, tenant_id, lagos, "store_manager").await;
    let (_, abuja_manager) = seed_staff(&pool, tenant_id, abuja, "store_manager").await;
    let (_, kano_manager) = seed_staff(&pool, tenant_id, kano, "store_manager").await;
    let (lagos_str, abuja_str, kano_str) = (lagos.to_string(), abuja.to_string(), kano.to_string());

    let (status, body) = post(
        &router,
        "/api/v1/stock/transfers",
        &at_store(&lagos_manager, &lagos_str),
        json!({ "from_store_id": lagos, "to_store_id": abuja, "product_id": rice, "stock_item_id": null, "quantity": "20", "reason": "Rebalance" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let transfer_id = id_of(&body);

    let abuja_inbox = inbox(&router, &at_store(&abuja_manager, &abuja_str)).await;
    assert_eq!(abuja_inbox.len(), 1);
    assert_eq!(abuja_inbox[0]["kind"], "transfer_requested");
    assert_eq!(abuja_inbox[0]["entity_type"], "stock_transfer");
    assert_eq!(abuja_inbox[0]["entity_id"], transfer_id);
    assert_eq!(abuja_inbox[0]["store_id"], abuja_str);
    assert!(abuja_inbox[0]["body"].as_str().unwrap().contains("Rice 50kg"));

    // The requester isn't told about their own action, and Kano has no part in it
    assert!(inbox(&router, &at_store(&lagos_manager, &lagos_str)).await.is_empty());
    assert!(inbox(&router, &at_store(&kano_manager, &kano_str)).await.is_empty());

    // The notification doesn't open Kano's transfers to the Abuja manager
    let (status, _) = get(&router, &format!("/api/v1/stock/transfers?store_id={}", kano), &at_store(&abuja_manager, &abuja_str)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, body) = post(&router, &format!("/api/v1/stock/transfers/{}/reject", transfer_id), &at_store(&abuja_manager, &abuja_str), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let lagos_inbox = inbox(&router, &at_store(&lagos_manager, &lagos_str)).await;
    assert_eq!(lagos_inbox.len(), 1);
    assert_eq!(lagos_inbox[0]["kind"], "transfer_rejected");
}

#[sqlx::test]
async fn notifications_are_marked_read_by_their_recipient_only(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "read-state-owner@example.com").await;
    let store = create_store(&router, &owner, "read-state").await;
    let (_, manager) = seed_staff(&pool, tenant_id, store, "store_manager").await;
    let (_, other_manager) = seed_staff(&pool, tenant_id, store, "store_manager").await;
    let store_str = store.to_string();
    let manager_caller = at_store(&manager, &store_str);

    let (_, body) = post(&router, "/api/v1/expense-categories", &as_user(&owner), json!({ "name": "Fuel" })).await;
    let category_id = id_of(&body);
    for _ in 0..2 {
        let (_, body) = post(
            &router,
            "/api/v1/expenses",
            &as_user(&owner),
            json!({ "store_id": store, "category_id": category_id, "amount_minor": 12_550, "expense_date": "2026-10-20", "description": "Diesel" }),
        )
        .await;
        let (status, _) = post(&router, &format!("/api/v1/expenses/{}/submit", id_of(&body)), &as_user(&owner), json!({})).await;
        assert_eq!(status, StatusCode::OK);
    }

    let (_, body) = get(&router, "/api/v1/notifications/unread-count", &manager_caller).await;
    assert_eq!(body["data"]["unread"], 2);
    let notifications = inbox(&router, &manager_caller).await;
    assert_eq!(notifications[0]["kind"], "expense_submitted");
    assert!(notifications[0]["body"].as_str().unwrap().contains("125.50"));
    let first = notifications[0]["id"].as_str().unwrap().to_string();

    let (status, _) = post(&router, &format!("/api/v1/notifications/{}/read", first), &at_store(&other_manager, &store_str), json!({})).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = post(&router, &format!("/api/v1/notifications/{}/read", Uuid::now_v7()), &manager_caller, json!({})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, body) = post(&router, &format!("/api/v1/notifications/{}/read", first), &manager_caller, json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "read");

    let (_, body) = get(&router, "/api/v1/notifications?status=unread", &manager_caller).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);

    let (_, body) = post(&router, "/api/v1/notifications/read-all", &manager_caller, json!({})).await;
    assert_eq!(body["data"]["updated"], 1);
    let (_, body) = get(&router, "/api/v1/notifications/unread-count", &manager_caller).await;
    assert_eq!(body["data"]["unread"], 0);

    // The other manager's copy is independent of the first manager's
    let (_, body) = get(&router, "/api/v1/notifications/unread-count", &at_store(&other_manager, &store_str)).await;
    assert_eq!(body["data"]["unread"], 1);
}

#[sqlx::test]
async fn a_failing_notification_does_not_roll_back_the_stock_change(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "notify-failure-owner@example.com").await;
    let store = create_store(&router, &owner, "notify-failure").await;
    let unit = seed_unit(&pool, tenant_id, "Bag", "bag").await;
    let rice = create_product(&router, &owner, unit, "Rice 50kg", "RICE-NF").await;
    set_threshold(&router, &owner, rice, store, "5").await;
    stock_up(&router, &owner, store, rice, "10").await;
    let (_, manager) = seed_staff(&pool, tenant_id, store, "store_manager").await;

    sqlx::query(
        "CREATE FUNCTION atlas.fail_notifications() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN RAISE EXCEPTION 'notification store down'; END $$",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("CREATE TRIGGER fail_notifications BEFORE INSERT ON atlas.notifications FOR EACH ROW EXECUTE FUNCTION atlas.fail_notifications()")
        .execute(&pool)
        .await
        .unwrap();

    adjust(&router, &owner, store, rice, "-8").await;
    assert_eq!(on_hand(&router, &owner, store, rice).await.parse::<f64>().unwrap(), 2.0);
    assert!(inbox(&router, &at_store(&manager, &store.to_string())).await.is_empty());
}
