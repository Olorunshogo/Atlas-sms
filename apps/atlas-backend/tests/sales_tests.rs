//! FR-15 point-of-sale checkout: server-priced sales, stock deducted through the inventory service, idempotent replays, offline sync and voids.

mod common;

use axum::http::StatusCode;
use common::*;
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn qty(value: &str) -> Decimal {
    return value.parse().unwrap();
}

async fn count(pool: &PgPool, sql: &str, id: Uuid) -> i64 {
    return sqlx::query_scalar(sql).bind(id).fetch_one(pool).await.unwrap();
}

#[sqlx::test]
async fn fr15_selling_two_bags_reduces_lagos_balance_to_98(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "fr15-owner@example.com").await;
    let lagos = create_store(&router, &owner, "lagos").await;
    let bag = seed_unit(&pool, tenant_id, "Bag", "bag").await;
    let rice = sellable(&router, &owner, lagos, bag, "Rice 50kg", "RICE-50", 75_000, "100").await;
    let (cashier_id, cashier) = seed_staff(&pool, tenant_id, lagos, "cashier").await;
    let shift = open_shift(&router, &cashier, lagos).await;
    let cash = method_id(&router, &owner, "cash").await;

    let request = sale_request(lagos, shift, vec![cart_line(rice, bag, "2")], vec![pay(cash, 150_000)]);
    let (status, body) = post(&router, "/api/v1/sales", &as_user(&cashier), request.clone()).await;
    assert_eq!(status, StatusCode::OK, "{}", body);

    assert_eq!(qty(&on_hand(&router, &owner, lagos, rice).await), qty("98"));

    let sale = &body["data"];
    let sale_id = Uuid::parse_str(sale["id"].as_str().unwrap()).unwrap();
    assert_eq!(sale["id"], request["id"]);
    assert_eq!(sale["store_id"], lagos.to_string());
    assert_eq!(sale["cashier_user_id"], cashier_id.to_string());
    assert_eq!(sale["status"], "completed");
    assert_eq!(sale["payment_status"], "paid");
    assert_eq!(sale["total_amount_minor"], 150_000);
    assert!(sale["created_at"].is_string());
    assert!(sale["completed_at"].is_string());
    let line = &sale["lines"][0];
    assert_eq!(sale["lines"].as_array().unwrap().len(), 1);
    assert_eq!(line["product_name"], "Rice 50kg");
    assert_eq!(qty(line["quantity"].as_str().unwrap()), qty("2"));
    assert_eq!(line["unit_price_amount_minor"], 75_000);
    assert_eq!(line["line_discount_amount_minor"], 0);
    assert_eq!(line["tax_amount_minor"], 0);
    assert_eq!(line["line_total_amount_minor"], 150_000);

    assert_eq!(count(&pool, "SELECT COUNT(*) FROM atlas.sales WHERE store_id = $1", lagos).await, 1);
    assert_eq!(
        count(&pool, "SELECT COUNT(*) FROM atlas.stock_movements WHERE reference_id = $1 AND movement_type = 'sale'", sale_id).await,
        1
    );
}

#[sqlx::test]
async fn a_live_sale_cannot_oversell_but_a_synced_one_is_recorded(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "oversell-owner@example.com").await;
    let store = create_store(&router, &owner, "lagos").await;
    let each = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let soap = sellable(&router, &owner, store, each, "Soap", "SOAP", 500, "100").await;
    let shift = open_shift(&router, &owner, store).await;
    let cash = method_id(&router, &owner, "cash").await;

    let request = sale_request(store, shift, vec![cart_line(soap, each, "101")], vec![pay(cash, 50_500)]);
    let (status, body) = post(&router, "/api/v1/sales", &as_user(&owner), request.clone()).await;
    assert_eq!(status, StatusCode::CONFLICT, "{}", body);
    assert_eq!(body["code"], "PRODUCT_OUT_OF_STOCK");
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM atlas.sales WHERE store_id = $1", store).await, 0);
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM atlas.payments WHERE tenant_id = $1", tenant_id).await, 0);

    let (status, body) = post(&router, "/api/v1/sync/sales", &as_user(&owner), request).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(qty(&on_hand(&router, &owner, store, soap).await), qty("-1"));
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM atlas.stock_exceptions WHERE store_id = $1", store).await, 1);
}

#[sqlx::test]
async fn replaying_a_sale_is_a_no_op_and_a_changed_replay_conflicts(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "replay-owner@example.com").await;
    let store = create_store(&router, &owner, "lagos").await;
    let each = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let soap = sellable(&router, &owner, store, each, "Soap", "SOAP", 500, "100").await;
    let shift = open_shift(&router, &owner, store).await;
    let cash = method_id(&router, &owner, "cash").await;
    let caller = as_user(&owner);

    let request = sale_request(store, shift, vec![cart_line(soap, each, "2")], vec![pay(cash, 1_000)]);
    let (first, second) = tokio::join!(
        post(&router, "/api/v1/sales", &caller, request.clone()),
        post(&router, "/api/v1/sync/sales", &caller, request.clone()),
    );
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    assert_eq!(second.0, StatusCode::OK, "{}", second.1);
    assert_eq!(first.1["data"]["id"], second.1["data"]["id"]);

    let sale_id = Uuid::parse_str(request["id"].as_str().unwrap()).unwrap();
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM atlas.sales WHERE id = $1", sale_id).await, 1);
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM atlas.stock_movements WHERE reference_id = $1", sale_id).await, 1);
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM atlas.payments WHERE sale_id = $1", sale_id).await, 1);
    assert_eq!(qty(&on_hand(&router, &owner, store, soap).await), qty("98"));

    let mut changed = request.clone();
    changed["lines"][0]["quantity"] = json!("3");
    changed["payments"][0]["amount_minor"] = json!(1_500);
    let (status, body) = post(&router, "/api/v1/sales", &caller, changed).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "IDEMPOTENCY_CONFLICT");
}

#[sqlx::test]
async fn price_overrides_need_the_permission_on_the_live_path(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "override-owner@example.com").await;
    let store = create_store(&router, &owner, "lagos").await;
    let each = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let soap = sellable(&router, &owner, store, each, "Soap", "SOAP", 500, "100").await;
    let (_, cashier) = seed_staff(&pool, tenant_id, store, "cashier").await;
    let shift = open_shift(&router, &owner, store).await;
    let cash = method_id(&router, &owner, "cash").await;

    let overridden = json!({
        "product_id": soap, "unit_id": each, "quantity": "1",
        "unit_price_amount_minor": 300, "price_override_reason": "Dented"
    });
    let request = sale_request(store, shift, vec![overridden.clone()], vec![pay(cash, 300)]);
    let (status, body) = post(&router, "/api/v1/sales", &as_user(&cashier), request).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{}", body);
    assert_eq!(body["code"], "PRICE_OVERRIDE_FORBIDDEN");

    let request = sale_request(store, shift, vec![overridden], vec![pay(cash, 300)]);
    let (status, body) = post(&router, "/api/v1/sales", &as_user(&owner), request).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["price_overridden"], true);
    assert_eq!(body["data"]["lines"][0]["list_unit_price_amount_minor"], 500);
    assert_eq!(body["data"]["lines"][0]["unit_price_amount_minor"], 300);
}

#[sqlx::test]
async fn checkout_applies_exclusive_tax_and_rejects_a_stale_total(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "tax-owner@example.com").await;
    let store = create_store(&router, &owner, "lagos").await;
    let each = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let vat = tax_rate(&router, &owner, "VAT", 750).await;
    store_settings(&router, &owner, store, json!({ "default_tax_rate_id": vat, "prices_include_tax": false })).await;
    let fan = sellable(&router, &owner, store, each, "Fan", "FAN", 20_000, "10").await;
    let shift = open_shift(&router, &owner, store).await;
    let card = method_id(&router, &owner, "card").await;

    let mut stale = sale_request(store, shift, vec![cart_line(fan, each, "2")], vec![pay(card, 43_000)]);
    stale["expected_total_amount_minor"] = json!(40_000);
    let (status, body) = post(&router, "/api/v1/sales", &as_user(&owner), stale).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "TOTAL_MISMATCH");

    let mut request = sale_request(store, shift, vec![cart_line(fan, each, "2")], vec![pay(card, 43_000)]);
    request["expected_total_amount_minor"] = json!(43_000);
    let (status, body) = post(&router, "/api/v1/sales", &as_user(&owner), request).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["subtotal_amount_minor"], 40_000);
    assert_eq!(body["data"]["tax_amount_minor"], 3_000);
    assert_eq!(body["data"]["total_amount_minor"], 43_000);
}

#[sqlx::test]
async fn selling_in_a_larger_unit_deducts_base_units(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "units-owner@example.com").await;
    let store = create_store(&router, &owner, "lagos").await;
    let kg = seed_unit(&pool, tenant_id, "Kilogram", "kg").await;
    let bag = seed_unit(&pool, tenant_id, "Bag 50kg", "bag").await;
    let rice = sellable(&router, &owner, store, kg, "Rice", "RICE", 1_500, "500").await;
    sale_unit(&router, &owner, rice, bag, "50").await;
    let shift = open_shift(&router, &owner, store).await;
    let cash = method_id(&router, &owner, "cash").await;

    let request = sale_request(store, shift, vec![cart_line(rice, bag, "1")], vec![pay(cash, 75_000)]);
    let (status, body) = post(&router, "/api/v1/sales", &as_user(&owner), request).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(qty(body["data"]["lines"][0]["base_quantity"].as_str().unwrap()), qty("50"));
    assert_eq!(qty(&on_hand(&router, &owner, store, rice).await), qty("450"));
}

#[sqlx::test]
async fn a_closed_shift_blocks_live_sales_but_not_synced_ones(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "shift-sale-owner@example.com").await;
    let store = create_store(&router, &owner, "lagos").await;
    let each = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let soap = sellable(&router, &owner, store, each, "Soap", "SOAP", 500, "100").await;
    let shift = open_shift(&router, &owner, store).await;
    let cash = method_id(&router, &owner, "cash").await;
    let (status, _) = post(&router, &format!("/api/v1/shifts/{}/close", shift), &as_user(&owner), json!({ "closing_cash_amount_minor": 0 })).await;
    assert_eq!(status, StatusCode::OK);

    let request = sale_request(store, shift, vec![cart_line(soap, each, "1")], vec![pay(cash, 500)]);
    let (status, body) = post(&router, "/api/v1/sales", &as_user(&owner), request.clone()).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "SHIFT_NOT_OPEN");

    let (status, body) = post(&router, "/api/v1/sync/sales", &as_user(&owner), request).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
}

#[sqlx::test]
async fn voiding_a_sale_restores_stock_and_reverses_payments(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "void-owner@example.com").await;
    let store = create_store(&router, &owner, "lagos").await;
    let each = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let soap = sellable(&router, &owner, store, each, "Soap", "SOAP", 500, "10").await;
    let (_, cashier) = seed_staff(&pool, tenant_id, store, "cashier").await;
    let shift = open_shift(&router, &owner, store).await;
    let cash = method_id(&router, &owner, "cash").await;

    let request = sale_request(store, shift, vec![cart_line(soap, each, "4")], vec![pay(cash, 2_000)]);
    let (status, body) = post(&router, "/api/v1/sales", &as_user(&cashier), request).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let sale_id = id_of(&body);
    assert_eq!(qty(&on_hand(&router, &owner, store, soap).await), qty("6"));

    let void_path = format!("/api/v1/sales/{}/void", sale_id);
    let (status, _) = post(&router, &void_path, &as_user(&cashier), json!({ "reason": "Rang up twice" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, body) = post(&router, &void_path, &as_user(&owner), json!({ "reason": "Rang up twice" })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "voided");
    assert_eq!(qty(&on_hand(&router, &owner, store, soap).await), qty("10"));
    let (status, payments) = get(&router, &format!("/api/v1/payments?sale_id={}", sale_id), &as_user(&owner)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(payments["data"][0]["status"], "reversed");

    let (status, _) = post(&router, &void_path, &as_user(&owner), json!({ "reason": "again" })).await;
    assert_eq!(status, StatusCode::CONFLICT);
}
