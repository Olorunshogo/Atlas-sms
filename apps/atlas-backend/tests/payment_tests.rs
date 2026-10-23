//! FR-16 payments: split tenders, pay-later, pending card payments, cash change, references, reversals, reconciliation and the shift drawer count.

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

fn transfer(method_id: Uuid, amount_minor: i64, reference: &str) -> Value {
    return json!({ "id": Uuid::now_v7(), "payment_method_id": method_id, "amount_minor": amount_minor, "reference": reference });
}

struct Shop {
    router: axum::Router,
    owner: String,
    tenant_id: Uuid,
    store: Uuid,
    shift: Uuid,
    each: Uuid,
    /// N5,000 per unit, 100 on hand.
    radio: Uuid,
    cash: Uuid,
    card: Uuid,
    transfer: Uuid,
}

async fn shop(pool: &PgPool, email: &str) -> Shop {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, pool, email).await;
    let store = create_store(&router, &owner, "lagos").await;
    let each = seed_unit(pool, tenant_id, "Each", "ea").await;
    let radio = sellable(&router, &owner, store, each, "Radio", "RADIO", 5_000, "100").await;
    let shift = open_shift(&router, &owner, store).await;
    let cash = method_id(&router, &owner, "cash").await;
    let card = method_id(&router, &owner, "card").await;
    let transfer = method_id(&router, &owner, "transfer").await;
    return Shop { router, owner, tenant_id, store, shift, each, radio, cash, card, transfer };
}

#[sqlx::test]
async fn fr16_split_payment_must_cover_balance_and_reconciles(pool: PgPool) {
    let s = shop(&pool, "fr16-owner@example.com").await;
    let caller = as_user(&s.owner);

    // N3,000 cash + N2,000 transfer covers a N5,000 sale.
    let split = sale_request(s.store, s.shift, vec![cart_line(s.radio, s.each, "1")], vec![pay(s.cash, 3_000), transfer(s.transfer, 2_000, "TRF-001")]);
    let (status, body) = post(&s.router, "/api/v1/sales", &caller, split).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "completed");
    assert_eq!(body["data"]["payment_status"], "paid");
    assert_eq!(body["data"]["payments"].as_array().unwrap().len(), 2);

    // N3,000 + N1,500 doesn't, and nothing is written.
    let short = sale_request(s.store, s.shift, vec![cart_line(s.radio, s.each, "1")], vec![pay(s.cash, 3_000), transfer(s.transfer, 1_500, "TRF-002")]);
    let short_id = Uuid::parse_str(short["id"].as_str().unwrap()).unwrap();
    let (status, body) = post(&s.router, "/api/v1/sales", &caller, short).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PAYMENT_INSUFFICIENT");
    let written: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM atlas.sales WHERE id = $1").bind(short_id).fetch_one(&pool).await.unwrap();
    assert_eq!(written, 0);
    assert_eq!(qty(&on_hand(&s.router, &s.owner, s.store, s.radio).await), qty("99"));

    // With pay-later on and a named customer, the same short tender completes as partially paid...
    store_settings(&s.router, &s.owner, s.store, json!({ "allow_pay_later": true })).await;
    let (status, customer) = post(&s.router, "/api/v1/customers", &caller, json!({ "full_name": "Emeka" })).await;
    assert_eq!(status, StatusCode::OK);
    let mut credit = sale_request(s.store, s.shift, vec![cart_line(s.radio, s.each, "1")], vec![pay(s.cash, 3_000), transfer(s.transfer, 1_500, "TRF-003")]);
    credit["customer_id"] = customer["data"]["id"].clone();
    let (status, body) = post(&s.router, "/api/v1/sales", &caller, credit).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "completed");
    assert_eq!(body["data"]["payment_status"], "partially_paid");
    let credit_sale = id_of(&body);

    // ...and a later N500 settles it; a further payment would overpay.
    let (status, body) = post(&s.router, &format!("/api/v1/sales/{}/payments", credit_sale), &caller, pay(s.cash, 500)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let (status, body) = post(&s.router, &format!("/api/v1/sales/{}/payments", credit_sale), &caller, pay(s.cash, 1)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PAYMENT_OVERPAID");
    let paid: String = sqlx::query_scalar("SELECT payment_status::text FROM atlas.sales WHERE id = $1::uuid")
        .bind(&credit_sale)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(paid, "paid");

    let (status, body) = get(&s.router, &format!("/api/v1/reports/payments/reconciliation?store_id={}", s.store), &caller).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let report = &body["data"];
    assert_eq!(report["variance_minor"], 0);
    assert_eq!(report["discrepancies"].as_array().unwrap().len(), 0);
    assert_eq!(report["sales"]["completed_count"], 2);
    assert_eq!(report["sales"]["amount_paid_minor"], 10_000);
    assert_eq!(report["payments_net_minor"], 10_000);
    let cash_row = report["by_method"].as_array().unwrap().iter().find(|m| m["kind"] == "cash").unwrap();
    assert_eq!(cash_row["collected_minor"], 6_500);
}

#[sqlx::test]
async fn a_pending_card_payment_holds_the_sale_until_it_succeeds(pool: PgPool) {
    let s = shop(&pool, "pending-owner@example.com").await;
    let (_, cashier) = seed_staff(&pool, s.tenant_id, s.store, "cashier").await;
    let caller = as_user(&s.owner);

    let mut pending = pay(s.card, 5_000);
    pending["status"] = json!("pending");
    let request = sale_request(s.store, s.shift, vec![cart_line(s.radio, s.each, "1")], vec![pending]);
    let (status, body) = post(&s.router, "/api/v1/sales", &as_user(&cashier), request).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "awaiting_payment");
    assert_eq!(qty(&on_hand(&s.router, &s.owner, s.store, s.radio).await), qty("100"));
    let payment_id = body["data"]["payments"][0]["id"].as_str().unwrap().to_string();

    let resolve = format!("/api/v1/payments/{}/status", payment_id);
    let (status, _) = post(&s.router, &resolve, &as_user(&cashier), json!({ "status": "succeeded" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, body) = post(&s.router, &resolve, &caller, json!({ "status": "succeeded", "reference": "AUTH-7781" })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "succeeded");
    let sale: (String, String) = sqlx::query_as("SELECT status::text, payment_status::text FROM atlas.sales WHERE id = $1::uuid")
        .bind(body["data"]["sale_id"].as_str().unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(sale, ("completed".to_string(), "paid".to_string()));
    assert_eq!(qty(&on_hand(&s.router, &s.owner, s.store, s.radio).await), qty("99"));

    // Already succeeded: failing it now is not a valid transition.
    let (status, body) = post(&s.router, &resolve, &caller, json!({ "status": "failed" })).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "INVALID_PAYMENT_TRANSITION");
}

#[sqlx::test]
async fn a_failed_card_payment_can_be_replaced_by_cash(pool: PgPool) {
    let s = shop(&pool, "failed-owner@example.com").await;
    let caller = as_user(&s.owner);
    let mut pending = pay(s.card, 5_000);
    pending["status"] = json!("pending");
    let request = sale_request(s.store, s.shift, vec![cart_line(s.radio, s.each, "1")], vec![pending]);
    let (status, body) = post(&s.router, "/api/v1/sales", &caller, request).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let sale_id = id_of(&body);
    let payment_id = body["data"]["payments"][0]["id"].as_str().unwrap().to_string();

    let (status, _) = post(&s.router, &format!("/api/v1/payments/{}/status", payment_id), &caller, json!({ "status": "failed", "reason": "Declined" })).await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = post(&s.router, &format!("/api/v1/sales/{}/payments", sale_id), &caller, pay(s.cash, 5_000)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let status_now: String = sqlx::query_scalar("SELECT status::text FROM atlas.sales WHERE id = $1::uuid").bind(&sale_id).fetch_one(&pool).await.unwrap();
    assert_eq!(status_now, "completed");
}

#[sqlx::test]
async fn cash_change_and_the_shift_drawer_count(pool: PgPool) {
    let s = shop(&pool, "change-owner@example.com").await;
    let caller = as_user(&s.owner);

    let mut cash = pay(s.cash, 5_000);
    cash["tendered_amount_minor"] = json!(10_000);
    let request = sale_request(s.store, s.shift, vec![cart_line(s.radio, s.each, "1")], vec![cash]);
    let (status, body) = post(&s.router, "/api/v1/sales", &caller, request).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["payments"][0]["amount_minor"], 5_000);
    assert_eq!(body["data"]["payments"][0]["change_amount_minor"], 5_000);

    // A card can't be tendered above the amount applied.
    let mut card = pay(s.card, 5_000);
    card["tendered_amount_minor"] = json!(6_000);
    let request = sale_request(s.store, s.shift, vec![cart_line(s.radio, s.each, "1")], vec![card]);
    let (status, _) = post(&s.router, "/api/v1/sales", &caller, request).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, body) = post(&s.router, &format!("/api/v1/shifts/{}/close", s.shift), &caller, json!({ "closing_cash_amount_minor": 4_500 })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["cash"]["expected_cash_minor"], 5_000);
    assert_eq!(body["data"]["cash"]["variance_minor"], -500);
}

#[sqlx::test]
async fn transfer_references_are_required_and_unique(pool: PgPool) {
    let s = shop(&pool, "reference-owner@example.com").await;
    let caller = as_user(&s.owner);

    let request = sale_request(s.store, s.shift, vec![cart_line(s.radio, s.each, "1")], vec![pay(s.transfer, 5_000)]);
    let (status, body) = post(&s.router, "/api/v1/sales", &caller, request).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PAYMENT_REFERENCE_REQUIRED");

    let request = sale_request(s.store, s.shift, vec![cart_line(s.radio, s.each, "1")], vec![transfer(s.transfer, 5_000, "TRF-9")]);
    let (status, _) = post(&s.router, "/api/v1/sales", &caller, request).await;
    assert_eq!(status, StatusCode::OK);
    let request = sale_request(s.store, s.shift, vec![cart_line(s.radio, s.each, "1")], vec![transfer(s.transfer, 5_000, "TRF-9")]);
    let (status, body) = post(&s.router, "/api/v1/sales", &caller, request).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "DUPLICATE_PAYMENT_REFERENCE");
}

#[sqlx::test]
async fn reversing_a_payment_reopens_the_balance(pool: PgPool) {
    let s = shop(&pool, "reverse-owner@example.com").await;
    let caller = as_user(&s.owner);
    let request = sale_request(s.store, s.shift, vec![cart_line(s.radio, s.each, "1")], vec![pay(s.card, 2_000), pay(s.cash, 3_000)]);
    let (status, body) = post(&s.router, "/api/v1/sales", &caller, request).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let sale_id = id_of(&body);
    let card_payment = body["data"]["payments"].as_array().unwrap().iter().find(|p| p["method"] == "card").unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, body) = post(&s.router, &format!("/api/v1/payments/{}/reverse", card_payment), &caller, json!({ "reason": "Wrong terminal" })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "reversed");
    let state: (String, i64) = sqlx::query_as("SELECT payment_status::text, amount_paid_minor FROM atlas.sales WHERE id = $1::uuid")
        .bind(&sale_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, ("partially_paid".to_string(), 3_000));

    let (status, body) = get(&s.router, &format!("/api/v1/reports/payments/reconciliation?store_id={}", s.store), &caller).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["variance_minor"], 0);
    assert_eq!(body["data"]["sales"]["outstanding_minor"], 2_000);
}
