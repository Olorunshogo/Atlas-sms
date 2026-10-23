//! FR-17 receipts and sales history: numbering, snapshots that reproduce the original values, permissioned reprints, and sales search.

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn transfer(method_id: Uuid, amount_minor: i64, reference: &str) -> Value {
    return json!({ "id": Uuid::now_v7(), "payment_method_id": method_id, "amount_minor": amount_minor, "reference": reference });
}

#[sqlx::test]
async fn fr17_receipt_totals_match_sale_and_payments(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "fr17-owner@example.com").await;
    let store = create_store(&router, &owner, "ikeja").await;
    let each = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let vat = tax_rate(&router, &owner, "VAT", 750).await;
    store_settings(&router, &owner, store, json!({ "default_tax_rate_id": vat, "receipt_footer": "Thank you" })).await;
    let iron = sellable(&router, &owner, store, each, "Pressing Iron", "IRON", 21_500, "10").await;
    let bulb = sellable(&router, &owner, store, each, "Bulb", "BULB", 1_075, "10").await;
    let shift = open_shift(&router, &owner, store).await;
    let cash = method_id(&router, &owner, "cash").await;
    let bank = method_id(&router, &owner, "transfer").await;

    let request = sale_request(
        store,
        shift,
        vec![cart_line(iron, each, "1"), cart_line(bulb, each, "2")],
        vec![pay(cash, 10_000), transfer(bank, 13_650, "TRF-55501234")],
    );
    let (status, sale) = post(&router, "/api/v1/sales", &as_user(&owner), request).await;
    assert_eq!(status, StatusCode::OK, "{}", sale);
    let sale = &sale["data"];
    let sale_id = sale["id"].as_str().unwrap();
    assert_eq!(sale["receipt_number"], "IKEJ-00000001");

    let (status, body) = get(&router, &format!("/api/v1/sales/{}/receipt", sale_id), &as_user(&owner)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let receipt = &body["data"];
    let snapshot = &receipt["snapshot"];
    assert_eq!(receipt["receipt_number"], sale["receipt_number"]);
    assert_eq!(snapshot["total_amount_minor"], sale["total_amount_minor"]);
    assert_eq!(snapshot["tax_amount_minor"], sale["tax_amount_minor"]);
    let line_sum: i64 = snapshot["lines"].as_array().unwrap().iter().map(|l| l["line_total_amount_minor"].as_i64().unwrap()).sum();
    assert_eq!(line_sum, sale["total_amount_minor"].as_i64().unwrap());
    let paid_sum: i64 = snapshot["payments"].as_array().unwrap().iter().map(|p| p["amount_minor"].as_i64().unwrap()).sum();
    assert_eq!(paid_sum, sale["amount_paid_minor"].as_i64().unwrap());
    assert_eq!(paid_sum, 23_650);
    assert_eq!(snapshot["cashier_name"], "Test Owner");
    assert_eq!(snapshot["customer_name"], "Walk-in customer");
    assert_eq!(snapshot["payments"][1]["reference"], "****1234");
    assert_eq!(snapshot["tax_breakdown"][0]["rate_basis_points"], 750);

    let (status, text) = call(&router, "GET", &format!("/api/v1/sales/{}/receipt?format=text&width=48", sale_id), Some(&as_user(&owner)), None).await;
    assert_eq!(status, StatusCode::OK);
    let text = text.as_str().unwrap();
    assert!(text.contains("IKEJ-00000001"), "{}", text);
    assert!(text.contains("236.50"), "{}", text);
    assert!(text.contains("Thank you"), "{}", text);
    assert!(!text.contains("REPRINT"));
}

#[sqlx::test]
async fn receipt_numbers_are_sequential_per_store(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "sequence-owner@example.com").await;
    let ikeja = create_store(&router, &owner, "ikeja").await;
    let lekki = create_store(&router, &owner, "lekki").await;
    store_settings(&router, &owner, lekki, json!({ "receipt_prefix": "lkk" })).await;
    let each = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let soap = sellable(&router, &owner, ikeja, each, "Soap", "SOAP", 500, "10").await;
    list_at(&router, &owner, soap, lekki).await;
    stock_up(&router, &owner, lekki, soap, "10").await;
    let cash = method_id(&router, &owner, "cash").await;
    let ikeja_shift = open_shift(&router, &owner, ikeja).await;
    let lekki_shift = open_shift(&router, &owner, lekki).await;

    let mut numbers = Vec::new();
    for (store, shift) in [(ikeja, ikeja_shift), (ikeja, ikeja_shift), (lekki, lekki_shift)] {
        let request = sale_request(store, shift, vec![cart_line(soap, each, "1")], vec![pay(cash, 500)]);
        let (status, body) = post(&router, "/api/v1/sales", &as_user(&owner), request).await;
        assert_eq!(status, StatusCode::OK, "{}", body);
        numbers.push(body["data"]["receipt_number"].as_str().unwrap().to_string());
    }
    assert_eq!(numbers, vec!["IKEJ-00000001", "IKEJ-00000002", "LKK-00000001"]);

    // A sale rejected for stock rolls its number back - no gap in the book.
    let too_many = sale_request(ikeja, ikeja_shift, vec![cart_line(soap, each, "50")], vec![pay(cash, 25_000)]);
    let (status, _) = post(&router, "/api/v1/sales", &as_user(&owner), too_many).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let request = sale_request(ikeja, ikeja_shift, vec![cart_line(soap, each, "1")], vec![pay(cash, 500)]);
    let (_, body) = post(&router, "/api/v1/sales", &as_user(&owner), request).await;
    assert_eq!(body["data"]["receipt_number"], "IKEJ-00000003");
}

#[sqlx::test]
async fn reprints_need_permission_and_are_marked(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "reprint-owner@example.com").await;
    let store = create_store(&router, &owner, "ikeja").await;
    let each = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let soap = sellable(&router, &owner, store, each, "Soap", "SOAP", 500, "10").await;
    let (_, cashier) = seed_staff(&pool, tenant_id, store, "cashier").await;
    let shift = open_shift(&router, &cashier, store).await;
    let cash = method_id(&router, &owner, "cash").await;

    let request = sale_request(store, shift, vec![cart_line(soap, each, "1")], vec![pay(cash, 500)]);
    let (status, body) = post(&router, "/api/v1/sales", &as_user(&cashier), request).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let sale_id = id_of(&body);
    let prints = format!("/api/v1/sales/{}/receipt/prints", sale_id);

    let (status, body) = post(&router, &prints, &as_user(&cashier), json!({ "device_id": "till-1" })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["kind"], "original");
    assert_eq!(body["data"]["copy_number"], 1);
    assert!(!body["data"]["text"].as_str().unwrap().contains("REPRINT"));

    let (status, _) = post(&router, &prints, &as_user(&cashier), json!({ "device_id": "till-1" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, body) = post(&router, &prints, &as_user(&owner), json!({ "device_id": "office", "reason": "Customer lost it" })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["kind"], "reprint");
    assert_eq!(body["data"]["copy_number"], 2);
    assert!(body["data"]["text"].as_str().unwrap().contains("*** REPRINT (copy 2) ***"));

    let audited: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM atlas.audit_events WHERE action = 'receipt.reprint' AND entity_id = $1::uuid")
        .bind(&sale_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(audited, 1);
}

#[sqlx::test]
async fn historical_receipts_keep_their_original_values(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "history-receipt-owner@example.com").await;
    let store = create_store(&router, &owner, "ikeja").await;
    store_settings(&router, &owner, store, json!({ "address": "1 Old Road" })).await;
    let each = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let soap = sellable(&router, &owner, store, each, "Soap", "SOAP", 500, "10").await;
    let shift = open_shift(&router, &owner, store).await;
    let cash = method_id(&router, &owner, "cash").await;
    let request = sale_request(store, shift, vec![cart_line(soap, each, "2")], vec![pay(cash, 1_000)]);
    let (status, body) = post(&router, "/api/v1/sales", &as_user(&owner), request).await;
    assert_eq!(status, StatusCode::OK);
    let sale_id = id_of(&body);

    // The world moves on: new price, new name, new address.
    price_rule(&router, &owner, soap, Some(store), 900).await;
    let (status, _) = patch(&router, &format!("/api/v1/products/{}", soap), &as_user(&owner), json!({ "name": "Premium Soap" })).await;
    assert_eq!(status, StatusCode::OK);
    store_settings(&router, &owner, store, json!({ "address": "99 New Avenue" })).await;

    let (status, body) = get(&router, &format!("/api/v1/sales/{}/receipt", sale_id), &as_user(&owner)).await;
    assert_eq!(status, StatusCode::OK);
    let snapshot = &body["data"]["snapshot"];
    assert_eq!(snapshot["store"]["address"], "1 Old Road");
    assert_eq!(snapshot["lines"][0]["name"], "Soap");
    assert_eq!(snapshot["lines"][0]["unit_price_amount_minor"], 500);
    assert_eq!(snapshot["total_amount_minor"], 1_000);
}

#[sqlx::test]
async fn sales_search_filters_and_respects_scope(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "search-owner@example.com").await;
    let ikeja = create_store(&router, &owner, "ikeja").await;
    let lekki = create_store(&router, &owner, "lekki").await;
    let each = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let soap = sellable(&router, &owner, ikeja, each, "Soap", "SOAP", 500, "100").await;
    list_at(&router, &owner, soap, lekki).await;
    stock_up(&router, &owner, lekki, soap, "100").await;
    let (cashier_id, cashier) = seed_staff(&pool, tenant_id, ikeja, "cashier").await;
    let (_, lekki_cashier) = seed_staff(&pool, tenant_id, lekki, "cashier").await;
    let (_, accountant) = seed_staff(&pool, tenant_id, ikeja, "accountant").await;
    let cash = method_id(&router, &owner, "cash").await;
    let card = method_id(&router, &owner, "card").await;
    let ikeja_shift = open_shift(&router, &cashier, ikeja).await;
    let lekki_shift = open_shift(&router, &lekki_cashier, lekki).await;

    let (_, by_cashier) = post(&router, "/api/v1/sales", &as_user(&cashier), sale_request(ikeja, ikeja_shift, vec![cart_line(soap, each, "1")], vec![pay(cash, 500)])).await;
    let (_, by_owner) = post(&router, "/api/v1/sales", &as_user(&owner), sale_request(ikeja, ikeja_shift, vec![cart_line(soap, each, "2")], vec![pay(card, 1_000)])).await;
    let (_, in_lekki) = post(&router, "/api/v1/sales", &as_user(&lekki_cashier), sale_request(lekki, lekki_shift, vec![cart_line(soap, each, "1")], vec![pay(cash, 500)])).await;
    let receipt = by_owner["data"]["receipt_number"].as_str().unwrap().to_string();

    let ids = |body: &Value| -> Vec<String> {
        return body["data"]["items"].as_array().unwrap().iter().map(|s| s["id"].as_str().unwrap().to_string()).collect();
    };

    let (status, body) = get(&router, "/api/v1/sales", &as_user(&owner)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(ids(&body).len(), 3);

    let (_, body) = get(&router, &format!("/api/v1/sales?store_id={}&cashier_user_id={}", ikeja, cashier_id), &as_user(&owner)).await;
    assert_eq!(ids(&body), vec![id_of(&by_cashier)]);

    let (_, body) = get(&router, &format!("/api/v1/sales?q={}", receipt), &as_user(&owner)).await;
    assert_eq!(ids(&body), vec![id_of(&by_owner)]);

    let prefix: String = id_of(&in_lekki).chars().take(24).collect();
    let (_, body) = get(&router, &format!("/api/v1/sales?q={}", prefix), &as_user(&owner)).await;
    assert_eq!(ids(&body), vec![id_of(&in_lekki)]);

    let (_, body) = get(&router, "/api/v1/sales?payment_status=paid&status=completed&from=2000-01-01T00:00:00Z", &as_user(&owner)).await;
    assert_eq!(ids(&body).len(), 3);
    let (_, body) = get(&router, "/api/v1/sales?to=2000-01-01T00:00:00Z", &as_user(&owner)).await;
    assert_eq!(ids(&body).len(), 0);

    // The accountant's sale.view reaches Ikeja only.
    let (status, body) = get(&router, "/api/v1/sales", &as_user(&accountant)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ids(&body).len(), 2);
    let (status, _) = get(&router, &format!("/api/v1/sales?store_id={}", lekki), &as_user(&accountant)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // A cashier without sale.view sees only their own sales in the open shift.
    let (_, body) = get(&router, "/api/v1/sales", &as_user(&cashier)).await;
    assert_eq!(ids(&body), vec![id_of(&by_cashier)]);
    let (status, _) = get(&router, &format!("/api/v1/sales/{}", id_of(&in_lekki)), &as_user(&cashier)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, body) = get(&router, &format!("/api/v1/sales/by-receipt/{}", receipt), &as_user(&owner)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], by_owner["data"]["id"]);
}
