//! FR-18 returns and refunds: quantity limits, restocking by condition, the return window, approvals, refund routes, and the books still balancing afterwards.

mod common;

use axum::http::StatusCode;
use common::*;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn qty(value: &str) -> Decimal {
    return value.parse().unwrap();
}

fn return_request(sale_line_id: &str, quantity: &str, condition: &str, refund_method: &str) -> Value {
    return json!({
        "id": Uuid::now_v7(),
        "device_id": "till-1",
        "reason_code": "changed_mind",
        "lines": [{ "sale_line_id": sale_line_id, "quantity": quantity, "condition": condition }],
        "refund": { "method": refund_method },
    });
}

struct Shop {
    router: axum::Router,
    owner: String,
    tenant_id: Uuid,
    store: Uuid,
    shift: Uuid,
    each: Uuid,
    /// N1,000.00 per unit, 100 on hand.
    kettle: Uuid,
    cash: Uuid,
    card: Uuid,
}

async fn shop(pool: &PgPool, email: &str) -> Shop {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, pool, email).await;
    let store = create_store(&router, &owner, "lagos").await;
    let each = seed_unit(pool, tenant_id, "Each", "ea").await;
    let kettle = sellable(&router, &owner, store, each, "Kettle", "KETTLE", 100_000, "100").await;
    let shift = open_shift(&router, &owner, store).await;
    let cash = method_id(&router, &owner, "cash").await;
    let card = method_id(&router, &owner, "card").await;
    return Shop { router, owner, tenant_id, store, shift, each, kettle, cash, card };
}

/// Sells `quantity` kettles paid by `payments` and returns (sale id, its single line id).
async fn sell(s: &Shop, token: &str, quantity: &str, payments: Vec<Value>, customer_id: Option<&str>) -> (String, String) {
    let mut request = sale_request(s.store, s.shift, vec![cart_line(s.kettle, s.each, quantity)], payments);
    if let Some(customer_id) = customer_id {
        request["customer_id"] = json!(customer_id);
    }
    let (status, body) = post(&s.router, "/api/v1/sales", &as_user(token), request).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    return (id_of(&body), body["data"]["lines"][0]["id"].as_str().unwrap().to_string());
}

#[sqlx::test]
async fn fr18_return_one_of_two_units_and_restock_only_when_eligible(pool: PgPool) {
    let s = shop(&pool, "fr18-owner@example.com").await;
    let caller = as_user(&s.owner);
    let (sale_id, line_id) = sell(&s, &s.owner, "2", vec![pay(s.cash, 200_000)], None).await;
    assert_eq!(qty(&on_hand(&s.router, &s.owner, s.store, s.kettle).await), qty("98"));
    let path = format!("/api/v1/sales/{}/returns", sale_id);

    // One resellable unit back: completes, goes back on the shelf, refunds half.
    let (status, body) = post(&s.router, &path, &caller, return_request(&line_id, "1", "resellable", "original")).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "completed");
    assert_eq!(body["data"]["refund_total_minor"], 100_000);
    assert_eq!(body["data"]["return_number"], "R-LAGO-00000001");
    assert_eq!(body["data"]["lines"][0]["restock"], true);
    assert_eq!(qty(&on_hand(&s.router, &s.owner, s.store, s.kettle).await), qty("99"));

    // Only one unit is left to return.
    let (status, body) = post(&s.router, &path, &caller, return_request(&line_id, "2", "resellable", "original")).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "RETURN_QUANTITY_EXCEEDED");

    // The last one comes back damaged: refunded, recorded, but stock doesn't rise.
    let (status, body) = post(&s.router, &path, &caller, return_request(&line_id, "1", "damaged", "original")).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["lines"][0]["restock"], false);
    assert_eq!(body["data"]["lines"][0]["stock_movement_id"], Value::Null);
    assert_eq!(qty(&on_hand(&s.router, &s.owner, s.store, s.kettle).await), qty("99"));

    let (status, body) = get(&s.router, &format!("/api/v1/sales/{}", sale_id), &caller).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["amount_refunded_minor"], 200_000);
    assert_eq!(body["data"]["payments"][0]["status"], "refunded");
    assert_eq!(body["data"]["refunds"].as_array().unwrap().len(), 2);

    let (status, body) = post(&s.router, &path, &caller, return_request(&line_id, "1", "resellable", "original")).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "RETURN_QUANTITY_EXCEEDED");
}

#[sqlx::test]
async fn partial_refunds_never_leak_a_kobo(pool: PgPool) {
    let s = shop(&pool, "kobo-owner@example.com").await;
    let caller = as_user(&s.owner);
    let mut line = cart_line(s.kettle, s.each, "3");
    line["discount"] = json!({ "amount_minor": 1, "reason": "Scuffed" });
    let request = sale_request(s.store, s.shift, vec![line], vec![pay(s.cash, 299_999)]);
    let (status, body) = post(&s.router, "/api/v1/sales", &caller, request).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let sale_id = id_of(&body);
    let line_id = body["data"]["lines"][0]["id"].as_str().unwrap().to_string();

    let mut refunds = Vec::new();
    for _ in 0..3 {
        let (status, body) = post(&s.router, &format!("/api/v1/sales/{}/returns", sale_id), &caller, return_request(&line_id, "1", "resellable", "original")).await;
        assert_eq!(status, StatusCode::OK, "{}", body);
        refunds.push(body["data"]["refund_total_minor"].as_i64().unwrap());
    }
    assert_eq!(refunds.iter().sum::<i64>(), 299_999);
}

#[sqlx::test]
async fn concurrent_returns_of_the_last_unit_only_one_wins(pool: PgPool) {
    let s = shop(&pool, "race-owner@example.com").await;
    let caller = as_user(&s.owner);
    let (sale_id, line_id) = sell(&s, &s.owner, "1", vec![pay(s.cash, 100_000)], None).await;
    let path = format!("/api/v1/sales/{}/returns", sale_id);

    let (a, b) = tokio::join!(
        post(&s.router, &path, &caller, return_request(&line_id, "1", "resellable", "original")),
        post(&s.router, &path, &caller, return_request(&line_id, "1", "resellable", "original")),
    );
    let statuses = [a.0, b.0];
    assert!(statuses.contains(&StatusCode::OK), "{:?} {:?}", a, b);
    assert!(statuses.contains(&StatusCode::CONFLICT), "{:?} {:?}", a, b);
    assert_eq!(qty(&on_hand(&s.router, &s.owner, s.store, s.kettle).await), qty("100"));
}

#[sqlx::test]
async fn a_return_outside_the_window_waits_for_someone_else_to_approve(pool: PgPool) {
    let s = shop(&pool, "window-owner@example.com").await;
    let (_, manager) = seed_staff(&pool, s.tenant_id, s.store, "store_manager").await;
    store_settings(&s.router, &s.owner, s.store, json!({ "return_window_days": 2 })).await;
    let (sale_id, line_id) = sell(&s, &manager, "2", vec![pay(s.cash, 200_000)], None).await;
    sqlx::query("UPDATE atlas.sales SET completed_at = now() - interval '5 days' WHERE id = $1::uuid")
        .bind(&sale_id)
        .execute(&pool)
        .await
        .unwrap();
    let path = format!("/api/v1/sales/{}/returns", sale_id);

    let (status, body) = post(&s.router, &path, &as_user(&manager), return_request(&line_id, "1", "resellable", "original")).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "pending_approval");
    assert_eq!(body["data"]["outside_window"], true);
    let pending_id = id_of(&body);
    assert_eq!(qty(&on_hand(&s.router, &s.owner, s.store, s.kettle).await), qty("98"));

    // The pending return holds its unit: only one more can be claimed.
    let (status, _) = post(&s.router, &path, &as_user(&manager), return_request(&line_id, "2", "resellable", "original")).await;
    assert_eq!(status, StatusCode::CONFLICT);

    let approve = format!("/api/v1/returns/{}/approve", pending_id);
    let (status, body) = post(&s.router, &approve, &as_user(&manager), json!({})).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "APPROVAL_SELF_FORBIDDEN");

    let (status, body) = post(&s.router, &approve, &as_user(&s.owner), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "completed");
    assert!(body["data"]["approved_by_user_id"].is_string());
    assert_eq!(body["data"]["refunds"][0]["approved_by_user_id"], body["data"]["approved_by_user_id"]);
    assert_eq!(qty(&on_hand(&s.router, &s.owner, s.store, s.kettle).await), qty("99"));

    // A rejected return frees its quantity again.
    let (status, body) = post(&s.router, &path, &as_user(&manager), return_request(&line_id, "1", "resellable", "original")).await;
    assert_eq!(status, StatusCode::OK);
    let second = id_of(&body);
    let (status, body) = post(&s.router, &format!("/api/v1/returns/{}/reject", second), &as_user(&s.owner), json!({ "reason": "Used item" })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "rejected");
    let (status, _) = post(&s.router, &path, &as_user(&manager), return_request(&line_id, "1", "resellable", "original")).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn cashier_returns_and_large_refunds_need_approval(pool: PgPool) {
    let s = shop(&pool, "approval-owner@example.com").await;
    let (_, cashier) = seed_staff(&pool, s.tenant_id, s.store, "cashier").await;
    let (_, manager) = seed_staff(&pool, s.tenant_id, s.store, "store_manager").await;
    store_settings(&s.router, &s.owner, s.store, json!({ "refund_approval_threshold_minor": 150_000 })).await;
    let (sale_id, line_id) = sell(&s, &cashier, "3", vec![pay(s.cash, 300_000)], None).await;
    let path = format!("/api/v1/sales/{}/returns", sale_id);

    // A cashier can start a return but can't authorise the refund.
    let (status, body) = post(&s.router, &path, &as_user(&cashier), return_request(&line_id, "1", "resellable", "original")).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "pending_approval");
    let (status, _) = post(&s.router, &format!("/api/v1/returns/{}/approve", id_of(&body)), &as_user(&cashier), json!({})).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = post(&s.router, &format!("/api/v1/returns/{}/approve", id_of(&body)), &as_user(&manager), json!({})).await;
    assert_eq!(status, StatusCode::OK);

    // A manager's return under the threshold completes; at or over it waits.
    let (status, body) = post(&s.router, &path, &as_user(&manager), return_request(&line_id, "1", "resellable", "original")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "completed");
    store_settings(&s.router, &s.owner, s.store, json!({ "refund_approval_threshold_minor": 100_000 })).await;
    let (status, body) = post(&s.router, &path, &as_user(&manager), return_request(&line_id, "1", "resellable", "original")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "pending_approval");
}

#[sqlx::test]
async fn refunds_to_store_credit_need_a_customer_and_build_a_ledger(pool: PgPool) {
    let s = shop(&pool, "credit-owner@example.com").await;
    let caller = as_user(&s.owner);
    let (_, walk_in_line) = sell(&s, &s.owner, "1", vec![pay(s.cash, 100_000)], None).await;
    let walk_in_sale: String = sqlx::query_scalar("SELECT sale_id::text FROM atlas.sale_lines WHERE id = $1::uuid")
        .bind(&walk_in_line)
        .fetch_one(&pool)
        .await
        .unwrap();
    let (status, body) = post(&s.router, &format!("/api/v1/sales/{}/returns", walk_in_sale), &caller, return_request(&walk_in_line, "1", "resellable", "store_credit")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "CUSTOMER_REQUIRED");

    let (_, customer) = post(&s.router, "/api/v1/customers", &caller, json!({ "full_name": "Ngozi" })).await;
    let customer_id = id_of(&customer);
    let (sale_id, line_id) = sell(&s, &s.owner, "2", vec![pay(s.cash, 200_000)], Some(&customer_id)).await;
    let (status, body) = post(&s.router, &format!("/api/v1/sales/{}/returns", sale_id), &caller, return_request(&line_id, "1", "resellable", "store_credit")).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["refunds"][0]["method_kind"], "store_credit");

    let (_, body) = get(&s.router, &format!("/api/v1/customers/{}", customer_id), &caller).await;
    assert_eq!(body["data"]["store_credit_amount_minor"], 100_000);
    let ledger: i64 = sqlx::query_scalar("SELECT SUM(amount_minor)::bigint FROM atlas.customer_credit_movements WHERE customer_id = $1::uuid")
        .bind(&customer_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(ledger, 100_000);
    // The cash payment is untouched - the money went to credit, not back to the drawer.
    let (_, body) = get(&s.router, &format!("/api/v1/sales/{}", sale_id), &caller).await;
    assert_eq!(body["data"]["payments"][0]["status"], "succeeded");

    let (status, body) = get(&s.router, &format!("/api/v1/reports/payments/reconciliation?store_id={}", s.store), &caller).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["variance_minor"], 0, "{}", body);
    assert_eq!(body["data"]["non_payment_refunds_minor"], 100_000);
}

#[sqlx::test]
async fn original_refunds_go_back_to_non_cash_payments_first(pool: PgPool) {
    let s = shop(&pool, "split-refund-owner@example.com").await;
    let caller = as_user(&s.owner);
    let (sale_id, line_id) = sell(&s, &s.owner, "5", vec![pay(s.cash, 200_000), pay(s.card, 300_000)], None).await;
    let path = format!("/api/v1/sales/{}/returns", sale_id);

    let (status, _) = post(&s.router, &path, &caller, return_request(&line_id, "1", "resellable", "original")).await;
    assert_eq!(status, StatusCode::OK);
    let (_, body) = get(&s.router, &format!("/api/v1/sales/{}", sale_id), &caller).await;
    let payments = body["data"]["payments"].as_array().unwrap();
    let card = payments.iter().find(|p| p["method"] == "card").unwrap();
    let cash = payments.iter().find(|p| p["method"] == "cash").unwrap();
    assert_eq!(card["status"], "partially_refunded");
    assert_eq!(card["refunded_amount_minor"], 100_000);
    assert_eq!(cash["refunded_amount_minor"], 0);

    let (status, _) = post(&s.router, &path, &caller, return_request(&line_id, "4", "resellable", "original")).await;
    assert_eq!(status, StatusCode::OK);
    let (_, body) = get(&s.router, &format!("/api/v1/sales/{}", sale_id), &caller).await;
    for payment in body["data"]["payments"].as_array().unwrap() {
        assert_eq!(payment["status"], "refunded");
    }

    // Nothing left to refund, so a step-up refund on the card is refused.
    let card_id = card["id"].as_str().unwrap();
    let (status, body) = post(&s.router, &format!("/api/v1/payments/{}/refund", card_id), &caller, json!({ "amount_minor": 1, "reason": "Goodwill" })).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "REFUND_AMOUNT_EXCEEDED");

    // Cash went back out of this shift's drawer.
    let (status, body) = post(&s.router, &format!("/api/v1/shifts/{}/close", s.shift), &caller, json!({ "closing_cash_amount_minor": 0 })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["cash"]["cash_refunded_minor"], 200_000);
    assert_eq!(body["data"]["cash"]["expected_cash_minor"], 0);

    let (_, body) = get(&s.router, &format!("/api/v1/reports/payments/reconciliation?store_id={}", s.store), &caller).await;
    assert_eq!(body["data"]["variance_minor"], 0, "{}", body);
    assert_eq!(body["data"]["discrepancies"].as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn a_replayed_return_is_recorded_once(pool: PgPool) {
    let s = shop(&pool, "return-replay-owner@example.com").await;
    let caller = as_user(&s.owner);
    let (sale_id, line_id) = sell(&s, &s.owner, "2", vec![pay(s.cash, 200_000)], None).await;
    let path = format!("/api/v1/sales/{}/returns", sale_id);
    let request = return_request(&line_id, "1", "resellable", "original");

    let (status, first) = post(&s.router, &path, &caller, request.clone()).await;
    assert_eq!(status, StatusCode::OK, "{}", first);
    let (status, second) = post(&s.router, &path, &caller, request).await;
    assert_eq!(status, StatusCode::OK, "{}", second);
    assert_eq!(first["data"]["return_number"], second["data"]["return_number"]);
    assert_eq!(qty(&on_hand(&s.router, &s.owner, s.store, s.kettle).await), qty("99"));

    let (status, text) = call(&s.router, "GET", &format!("/api/v1/returns/{}/receipt", id_of(&first)), Some(&caller), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(text.as_str().unwrap().contains("R-LAGO-00000001"));

    // Voiding is closed once goods have come back.
    let (status, _) = post(&s.router, &format!("/api/v1/sales/{}/void", sale_id), &caller, json!({ "reason": "Oops" })).await;
    assert_eq!(status, StatusCode::CONFLICT);
}
