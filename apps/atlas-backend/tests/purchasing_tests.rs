//! FR-20 to FR-23: supplier records and statements, the approval step, charges, over-receipt exceptions, partial cancellation, and returns that wait for confirmation.

mod common;

use axum::http::StatusCode;
use axum::Router;
use common::*;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn qty(value: &Value) -> Decimal {
    return value.as_str().unwrap().parse().unwrap();
}

struct Shop {
    router: Router,
    owner: String,
    tenant_id: Uuid,
    store: Uuid,
    rice: Uuid,
}

async fn shop(pool: &PgPool, email: &str) -> Shop {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, pool, email).await;
    let store = create_store(&router, &owner, "purchasing").await;
    let unit = seed_unit(pool, tenant_id, "Bag", "bag").await;
    let rice = create_product(&router, &owner, unit, "Rice 50kg", "RICE-P").await;
    return Shop { router, owner, tenant_id, store, rice };
}

async fn supplier(s: &Shop, body: Value) -> String {
    let (status, body) = post(&s.router, "/api/v1/suppliers", &as_user(&s.owner), body).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    return id_of(&body);
}

async fn order(s: &Shop, token: &str, supplier_id: &str, quantity: &str, charges: Value) -> String {
    let (status, body) = post(
        &s.router,
        "/api/v1/purchase-orders",
        &at_store(token, &s.store.to_string()),
        json!({
            "store_id": s.store,
            "supplier_id": supplier_id,
            "lines": [{ "product_id": s.rice, "quantity_ordered": quantity, "unit_cost_amount_minor": 500 }],
            "charges": charges,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    return id_of(&body);
}

async fn step(s: &Shop, token: &str, order_id: &str, action: &str) -> (StatusCode, Value) {
    return post(&s.router, &format!("/api/v1/purchase-orders/{}/{}", order_id, action), &at_store(token, &s.store.to_string()), json!({})).await;
}

async fn line_id(s: &Shop, order_id: &str) -> String {
    let (_, body) = get(&s.router, &format!("/api/v1/purchase-orders/{}/lines", order_id), &as_user(&s.owner)).await;
    return body["data"][0]["id"].as_str().unwrap().to_string();
}

async fn receive(s: &Shop, token: &str, order_id: &str, line: &str, quantity: &str, reason: Option<&str>) -> (StatusCode, Value) {
    return post(
        &s.router,
        &format!("/api/v1/purchase-orders/{}/receive", order_id),
        &at_store(token, &s.store.to_string()),
        json!({ "lines": [{ "purchase_order_line_id": line, "quantity_received": quantity, "over_receipt_reason": reason }] }),
    )
    .await;
}

async fn balance(s: &Shop, supplier_id: &str) -> i64 {
    let (_, body) = get(&s.router, &format!("/api/v1/suppliers/{}", supplier_id), &as_user(&s.owner)).await;
    return body["data"]["balance_amount_minor"].as_i64().unwrap();
}

/// A buyer who can raise orders at the store but not approve them.
async fn buyer(pool: &PgPool, s: &Shop) -> String {
    let role_id = Uuid::now_v7();
    sqlx::query("INSERT INTO atlas.roles (id, tenant_id, name, permissions) VALUES ($1, $2, 'buyer', ARRAY['purchase.create']::atlas.permission[])")
        .bind(role_id)
        .bind(s.tenant_id)
        .execute(pool)
        .await
        .unwrap();
    let user_id = Uuid::now_v7();
    sqlx::query("INSERT INTO atlas.users (id, full_name, email, password_hash, is_email_verified) VALUES ($1, 'Buyer', $2, 'x', true)")
        .bind(user_id)
        .bind(format!("buyer-{}@example.com", user_id))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO atlas.role_grants (id, user_id, role_id, scope, tenant_id, store_id, status, granted_by_user_id)
         VALUES ($1, $2, $3, 'store', $4, $5, 'active', $2)",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(role_id)
    .bind(s.tenant_id)
    .bind(s.store)
    .execute(pool)
    .await
    .unwrap();
    return issue_token(user_id, s.tenant_id);
}

// === FR-20 suppliers

#[sqlx::test]
async fn a_supplier_record_holds_its_details_products_and_status(pool: PgPool) {
    let s = shop(&pool, "supplier-record@example.com").await;
    let caller = as_user(&s.owner);
    let acme = supplier(&s, json!({ "name": "Acme Foods", "code": "ACM-01", "address": "12 Wharf Rd, Apapa", "payment_terms_days": 30, "phone": "+2348011111111" })).await;
    supplier(&s, json!({ "name": "Bright Oils" })).await;

    let (status, _) = post(&s.router, "/api/v1/suppliers", &caller, json!({ "name": "Copycat", "code": "acm-01" })).await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (_, body) = get(&s.router, "/api/v1/suppliers?q=acm", &caller).await;
    let found = body["data"].as_array().unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0]["payment_terms_days"], 30);
    assert_eq!(found[0]["address"], "12 Wharf Rd, Apapa");

    let (status, body) = call(
        &s.router,
        "PUT",
        &format!("/api/v1/suppliers/{}/products", acme),
        Some(&caller),
        Some(json!({ "products": [{ "product_id": s.rice, "supplier_sku": "AC-RICE", "unit_cost_amount_minor": 480 }] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let (_, body) = get(&s.router, &format!("/api/v1/suppliers/{}", acme), &caller).await;
    assert_eq!(body["data"]["code"], "ACM-01");
    assert_eq!(body["data"]["products"][0]["supplier_sku"], "AC-RICE");
    assert_eq!(body["data"]["products"][0]["product_name"], "Rice 50kg");

    let (status, body) = post(&s.router, &format!("/api/v1/suppliers/{}/deactivate", acme), &caller, json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"]["deactivated_at"].is_string());
    let (_, body) = get(&s.router, "/api/v1/suppliers", &caller).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    let (_, body) = get(&s.router, "/api/v1/suppliers?include_inactive=true", &caller).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 2);

    let (status, body) = post(
        &s.router,
        "/api/v1/purchase-orders",
        &caller,
        json!({ "store_id": s.store, "supplier_id": acme, "lines": [{ "product_id": s.rice, "quantity_ordered": "1", "unit_cost_amount_minor": 500 }] }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "SUPPLIER_INACTIVE");

    post(&s.router, &format!("/api/v1/suppliers/{}/reactivate", acme), &caller, json!({})).await;
    order(&s, &s.owner, &acme, "1", json!([])).await;
}

#[sqlx::test]
async fn a_supplier_statement_shows_only_its_own_history_and_adds_up_to_the_balance(pool: PgPool) {
    let s = shop(&pool, "supplier-statement@example.com").await;
    let acme = supplier(&s, json!({ "name": "Acme" })).await;
    let other = supplier(&s, json!({ "name": "Other" })).await;

    let acme_order = order(&s, &s.owner, &acme, "10", json!([{ "description": "Delivery", "amount_minor": 1_000 }])).await;
    step(&s, &s.owner, &acme_order, "submit").await;
    step(&s, &s.owner, &acme_order, "approve").await;
    receive(&s, &s.owner, &acme_order, &line_id(&s, &acme_order).await, "10", None).await;
    let other_order = order(&s, &s.owner, &other, "4", json!([])).await;
    step(&s, &s.owner, &other_order, "submit").await;
    step(&s, &s.owner, &other_order, "approve").await;
    receive(&s, &s.owner, &other_order, &line_id(&s, &other_order).await, "4", None).await;

    let (status, body) = post(
        &s.router,
        &format!("/api/v1/suppliers/{}/payments", acme),
        &as_user(&s.owner),
        json!({ "amount_minor": 2_500, "method": "bank_transfer", "reference": "TRF-1", "paid_on": "2026-10-24" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    // 10 x 500 received + 1,000 delivery - 2,500 paid
    assert_eq!(balance(&s, &acme).await, 3_500);

    let (status, body) = get(&s.router, &format!("/api/v1/suppliers/{}/transactions", acme), &as_user(&s.owner)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let history = body["data"]["transactions"].as_array().unwrap();
    let kinds: Vec<&str> = history.iter().map(|t| t["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, vec!["payment", "goods_receipt", "purchase_order"]);
    assert!(history.iter().all(|t| t["purchase_order_id"].is_null() || t["purchase_order_id"] == acme_order));
    let effect: i64 = history.iter().map(|t| t["balance_effect_minor"].as_i64().unwrap()).sum();
    assert_eq!(effect, body["data"]["balance_amount_minor"].as_i64().unwrap());
    assert_eq!(history[2]["amount_minor"], 6_000);
}

// === FR-21 approval, charges, receipts

#[sqlx::test]
async fn an_order_needs_an_approver_before_it_can_be_received(pool: PgPool) {
    let s = shop(&pool, "po-approval@example.com").await;
    let buyer = buyer(&pool, &s).await;
    let (_, manager) = seed_staff(&pool, s.tenant_id, s.store, "store_manager").await;
    let acme = supplier(&s, json!({ "name": "Acme" })).await;
    let order_id = order(&s, &buyer, &acme, "10", json!([])).await;
    let line = line_id(&s, &order_id).await;

    let (status, body) = receive(&s, &buyer, &order_id, &line, "1", None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "PURCHASE_ORDER_NOT_RECEIVABLE");

    let (status, body) = step(&s, &buyer, &order_id, "submit").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "submitted");
    let (_, inbox) = get(&s.router, "/api/v1/notifications", &at_store(&manager, &s.store.to_string())).await;
    assert_eq!(inbox["data"][0]["kind"], "purchase_order_submitted");

    let (status, _) = step(&s, &buyer, &order_id, "approve").await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Editing a submitted order sends it back to draft
    let (status, body) = patch(&s.router, &format!("/api/v1/purchase-orders/{}", order_id), &at_store(&buyer, &s.store.to_string()), json!({ "charges": [{ "description": "Loading", "amount_minor": 300 }] })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "draft");
    let (status, _) = step(&s, &manager, &order_id, "approve").await;
    assert_eq!(status, StatusCode::CONFLICT);

    step(&s, &buyer, &order_id, "submit").await;
    let (status, body) = step(&s, &manager, &order_id, "approve").await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "ordered");
    let (_, inbox) = get(&s.router, "/api/v1/notifications", &at_store(&buyer, &s.store.to_string())).await;
    assert_eq!(inbox["data"][0]["kind"], "purchase_order_approved");

    let (_, detail) = get(&s.router, &format!("/api/v1/purchase-orders/{}", order_id), &as_user(&s.owner)).await;
    assert!(detail["data"]["approved_at"].is_string());
    assert_eq!(detail["data"]["lines_total_amount_minor"], 5_000);
    assert_eq!(detail["data"]["charges_total_amount_minor"], 300);
    assert_eq!(detail["data"]["total_amount_minor"], 5_300);
}

#[sqlx::test]
async fn charges_are_owed_once_with_the_first_receipt(pool: PgPool) {
    let s = shop(&pool, "po-charges@example.com").await;
    let acme = supplier(&s, json!({ "name": "Acme" })).await;
    let order_id = order(&s, &s.owner, &acme, "10", json!([{ "description": "Delivery", "amount_minor": 1_000 }])).await;
    step(&s, &s.owner, &order_id, "submit").await;
    step(&s, &s.owner, &order_id, "approve").await;
    let line = line_id(&s, &order_id).await;

    receive(&s, &s.owner, &order_id, &line, "4", None).await;
    assert_eq!(balance(&s, &acme).await, 4 * 500 + 1_000);
    receive(&s, &s.owner, &order_id, &line, "6", None).await;
    assert_eq!(balance(&s, &acme).await, 10 * 500 + 1_000);
}

#[sqlx::test]
async fn only_an_approver_with_a_reason_can_receive_more_than_was_ordered(pool: PgPool) {
    let s = shop(&pool, "po-over-receipt@example.com").await;
    let buyer = buyer(&pool, &s).await;
    let acme = supplier(&s, json!({ "name": "Acme" })).await;
    let order_id = order(&s, &s.owner, &acme, "10", json!([])).await;
    step(&s, &s.owner, &order_id, "submit").await;
    step(&s, &s.owner, &order_id, "approve").await;
    let line = line_id(&s, &order_id).await;

    let (status, _) = receive(&s, &buyer, &order_id, &line, "12", Some("Supplier sent extra")).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = receive(&s, &s.owner, &order_id, &line, "12", None).await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, body) = receive(&s, &s.owner, &order_id, &line, "12", Some("Supplier sent extra")).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["purchase_order"]["status"], "received");
    let recorded: Option<String> = sqlx::query_scalar("SELECT over_receipt_reason FROM atlas.goods_received_note_lines WHERE purchase_order_line_id = $1::uuid")
        .bind(&line)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(recorded.as_deref(), Some("Supplier sent extra"));
    assert_eq!(qty(&Value::String(on_hand(&s.router, &s.owner, s.store, s.rice).await)), "12".parse().unwrap());
}

#[sqlx::test]
async fn cancelling_a_partly_received_order_cancels_only_the_rest(pool: PgPool) {
    let s = shop(&pool, "po-partial-cancel@example.com").await;
    let acme = supplier(&s, json!({ "name": "Acme" })).await;
    let order_id = order(&s, &s.owner, &acme, "10", json!([])).await;
    step(&s, &s.owner, &order_id, "submit").await;
    step(&s, &s.owner, &order_id, "approve").await;
    let line = line_id(&s, &order_id).await;
    receive(&s, &s.owner, &order_id, &line, "6", None).await;

    let (status, body) = step(&s, &s.owner, &order_id, "cancel").await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "cancelled");

    let (_, detail) = get(&s.router, &format!("/api/v1/purchase-orders/{}", order_id), &as_user(&s.owner)).await;
    let line_detail = &detail["data"]["lines"][0];
    assert_eq!(qty(&line_detail["quantity_received"]), "6".parse().unwrap());
    assert_eq!(qty(&line_detail["quantity_cancelled"]), "4".parse().unwrap());
    assert_eq!(qty(&line_detail["quantity_outstanding"]), Decimal::ZERO);
    assert_eq!(qty(&Value::String(on_hand(&s.router, &s.owner, s.store, s.rice).await)), "6".parse().unwrap());

    let (status, body) = receive(&s, &s.owner, &order_id, &line, "1", None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "PURCHASE_ORDER_NOT_RECEIVABLE");
}

// === FR-23 returns

#[sqlx::test]
async fn a_pending_return_holds_its_units_until_confirmed_or_cancelled(pool: PgPool) {
    let s = shop(&pool, "po-return-confirm@example.com").await;
    let caller = as_user(&s.owner);
    let acme = supplier(&s, json!({ "name": "Acme" })).await;
    let order_id = order(&s, &s.owner, &acme, "10", json!([])).await;
    step(&s, &s.owner, &order_id, "submit").await;
    step(&s, &s.owner, &order_id, "approve").await;
    let line = line_id(&s, &order_id).await;
    receive(&s, &s.owner, &order_id, &line, "10", None).await;
    let grn_line: String = sqlx::query_scalar("SELECT id::text FROM atlas.goods_received_note_lines WHERE purchase_order_line_id = $1::uuid")
        .bind(&line)
        .fetch_one(&pool)
        .await
        .unwrap();
    let returning = |quantity: &str| json!({ "purchase_order_id": order_id, "reason": "Torn bags", "lines": [{ "goods_received_note_line_id": grn_line, "quantity": quantity }] });

    let (status, first) = post(&s.router, "/api/v1/purchase-returns", &caller, returning("7")).await;
    assert_eq!(status, StatusCode::OK, "{}", first);
    // The 7 pending units can't be claimed again
    let (status, body) = post(&s.router, "/api/v1/purchase-returns", &caller, returning("4")).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "RETURN_QUANTITY_EXCEEDED");

    let first_id = id_of(&first);
    let (status, body) = post(&s.router, &format!("/api/v1/purchase-returns/{}/cancel", first_id), &caller, json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "cancelled");
    let (status, _) = post(&s.router, &format!("/api/v1/purchase-returns/{}/confirm", first_id), &caller, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(balance(&s, &acme).await, 5_000);

    let (_, second) = post(&s.router, "/api/v1/purchase-returns", &caller, returning("4")).await;
    let (status, body) = post(&s.router, &format!("/api/v1/purchase-returns/{}/confirm", id_of(&second)), &caller, json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["total_amount_minor"], 2_000);
    assert_eq!(balance(&s, &acme).await, 3_000);
    assert_eq!(qty(&Value::String(on_hand(&s.router, &s.owner, s.store, s.rice).await)), "6".parse().unwrap());

    let (_, body) = get(&s.router, &format!("/api/v1/purchase-returns?store_id={}&supplier_id={}", s.store, acme), &caller).await;
    let statuses: Vec<&str> = body["data"].as_array().unwrap().iter().map(|r| r["status"].as_str().unwrap()).collect();
    assert_eq!(statuses, vec!["confirmed", "cancelled"]);
}
