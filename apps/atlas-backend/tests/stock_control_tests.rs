//! FR-12 physical counts and FR-14 batches: variance before approval, first-expiry-first-out sales, expired stock, and unsellable goods kept apart.

mod common;

use axum::http::StatusCode;
use axum::Router;
use chrono::{Duration, Utc};
use common::*;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn qty(value: &str) -> Decimal {
    return value.parse().unwrap();
}

fn days_from_today(days: i64) -> String {
    return (Utc::now().date_naive() + Duration::days(days)).to_string();
}

// === Physical counts (FR-12)

#[sqlx::test]
async fn a_count_variance_is_visible_before_approval_and_posted_only_after(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "count-owner@example.com").await;
    let store = create_store(&router, &owner, "count-store").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let soap = create_product(&router, &owner, unit, "Soap", "SOAP-C").await;
    let salt = create_product(&router, &owner, unit, "Salt", "SALT-C").await;
    stock_up(&router, &owner, store, soap, "20").await;
    stock_up(&router, &owner, store, salt, "5").await;
    let (_, clerk) = seed_staff(&pool, tenant_id, store, "stock_clerk").await;
    let (_, cashier) = seed_staff(&pool, tenant_id, store, "cashier").await;
    let store_str = store.to_string();

    let count_body = json!({
        "store_id": store,
        "reason": "Month-end count",
        "lines": [
            { "product_id": soap, "stock_item_id": null, "counted_quantity": "17" },
            { "product_id": salt, "stock_item_id": null, "counted_quantity": "5" },
        ],
    });
    let (status, _) = post(&router, "/api/v1/stock/counts", &at_store(&cashier, &store_str), count_body.clone()).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, body) = post(&router, "/api/v1/stock/counts", &at_store(&clerk, &store_str), count_body).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "pending");
    assert_eq!(qty(body["data"]["lines"][0]["recorded_quantity"].as_str().unwrap()), qty("20"));
    assert_eq!(qty(body["data"]["lines"][0]["variance"].as_str().unwrap()), qty("-3"));
    assert_eq!(qty(body["data"]["lines"][1]["variance"].as_str().unwrap()), qty("0"));
    let count_id = id_of(&body);

    // Nothing moves while it waits
    assert_eq!(qty(&on_hand(&router, &owner, store, soap).await), qty("20"));
    let (_, body) = get(&router, "/api/v1/notifications", &as_user(&owner)).await;
    assert_eq!(body["data"][0]["kind"], "stock_count_submitted");

    let (status, body) = post(&router, &format!("/api/v1/stock/counts/{}/approve", count_id), &as_user(&owner), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "approved");
    assert_eq!(qty(&on_hand(&router, &owner, store, soap).await), qty("17"));
    assert_eq!(qty(&on_hand(&router, &owner, store, salt).await), qty("5"));

    let (status, body) = post(&router, &format!("/api/v1/stock/counts/{}/approve", count_id), &as_user(&owner), json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "STOCK_COUNT_NOT_PENDING");

    let (_, body) = get(&router, &format!("/api/v1/stock/movements?store_id={}&product_id={}", store, soap), &as_user(&owner)).await;
    assert!(body["data"].as_array().unwrap().iter().any(|m| m["reason"] == "Stock count: Month-end count"), "{}", body);
}

#[sqlx::test]
async fn a_rejected_count_changes_no_stock(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "count-reject-owner@example.com").await;
    let store = create_store(&router, &owner, "count-reject").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let soap = create_product(&router, &owner, unit, "Soap", "SOAP-R").await;
    stock_up(&router, &owner, store, soap, "20").await;

    let (_, body) = post(
        &router,
        "/api/v1/stock/counts",
        &as_user(&owner),
        json!({ "store_id": store, "reason": "Spot check", "lines": [{ "product_id": soap, "stock_item_id": null, "counted_quantity": "2" }] }),
    )
    .await;
    let (status, body) = post(&router, &format!("/api/v1/stock/counts/{}/reject", id_of(&body)), &as_user(&owner), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "rejected");
    assert_eq!(qty(&on_hand(&router, &owner, store, soap).await), qty("20"));

    let (_, body) = get(&router, &format!("/api/v1/stock/counts?store_id={}", store), &as_user(&owner)).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
}

// === Batches and expiry (FR-14)

struct Pharmacy {
    router: Router,
    owner: String,
    store: Uuid,
    unit: Uuid,
    drug: Uuid,
    shift: Uuid,
    cash: Uuid,
}

async fn pharmacy(pool: &PgPool, email: &str) -> Pharmacy {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, pool, email).await;
    let store = create_store(&router, &owner, "pharmacy").await;
    let unit = seed_unit(pool, tenant_id, "Pack", "pk").await;
    let (status, body) = post(&router, "/api/v1/product-types", &as_user(&owner), json!({ "name": "Medicine", "identity_mode": "by_batch" })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let (status, body) = post(
        &router,
        "/api/v1/products",
        &as_user(&owner),
        json!({ "product_type_id": id_of(&body), "base_unit_id": unit, "name": "Paracetamol", "sku": "PARA" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let drug = Uuid::parse_str(&id_of(&body)).unwrap();
    list_at(&router, &owner, drug, store).await;
    price_rule(&router, &owner, drug, None, 500).await;
    let shift = open_shift(&router, &owner, store).await;
    let cash = method_id(&router, &owner, "cash").await;
    return Pharmacy { router, owner, store, unit, drug, shift, cash };
}

async fn batch(p: &Pharmacy, number: &str, expiry_days: i64, quantity: &str) -> Uuid {
    let (status, body) = post(
        &p.router,
        "/api/v1/stock/batches",
        &as_user(&p.owner),
        json!({ "store_id": p.store, "product_id": p.drug, "batch_number": number, "expiry_date": days_from_today(expiry_days) }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let id = Uuid::parse_str(&id_of(&body)).unwrap();
    let (status, body) = post(
        &p.router,
        "/api/v1/stock/adjustments",
        &as_user(&p.owner),
        json!({ "store_id": p.store, "product_id": p.drug, "stock_item_id": id, "quantity": quantity, "reason": "Opening stock" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    return id;
}

async fn batch_on_hand(p: &Pharmacy) -> Vec<(String, Decimal)> {
    let (status, body) = get(&p.router, &format!("/api/v1/stock/batches?store_id={}", p.store), &as_user(&p.owner)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    return body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| (b["batch_number"].as_str().unwrap().to_string(), qty(b["on_hand"].as_str().unwrap())))
        .collect();
}

async fn sell(p: &Pharmacy, line: Value, amount: i64) -> (StatusCode, Value) {
    return post(&p.router, "/api/v1/sales", &as_user(&p.owner), sale_request(p.store, p.shift, vec![line], vec![pay(p.cash, amount)])).await;
}

#[sqlx::test]
async fn a_batch_tracked_product_must_name_its_batch(pool: PgPool) {
    let p = pharmacy(&pool, "batch-required-owner@example.com").await;
    let (status, body) = post(
        &p.router,
        "/api/v1/stock/adjustments",
        &as_user(&p.owner),
        json!({ "store_id": p.store, "product_id": p.drug, "stock_item_id": null, "quantity": "5", "reason": "Opening" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "BATCH_REQUIRED");
}

#[sqlx::test]
async fn sales_take_the_earliest_expiring_batch_first_and_skip_expired_stock(pool: PgPool) {
    let p = pharmacy(&pool, "fefo-owner@example.com").await;
    let expired = batch(&p, "B-OLD", -5, "10").await;
    batch(&p, "B-SOON", 20, "4").await;
    batch(&p, "B-LATE", 300, "10").await;

    let (status, body) = sell(&p, cart_line(p.drug, p.unit, "6"), 3_000).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let sale_id = id_of(&body);
    assert_eq!(
        batch_on_hand(&p).await,
        vec![("B-OLD".to_string(), qty("10")), ("B-LATE".to_string(), qty("8"))]
    );

    // Naming the expired batch is refused while the store forbids it
    let mut line = cart_line(p.drug, p.unit, "1");
    line["stock_item_id"] = json!(expired);
    let (status, body) = sell(&p, line.clone(), 500).await;
    assert_eq!(status, StatusCode::CONFLICT, "{}", body);
    assert_eq!(body["code"], "BATCH_EXPIRED");

    store_settings(&p.router, &p.owner, p.store, json!({ "allow_expired_sale": true })).await;
    let (status, body) = sell(&p, line, 500).await;
    assert_eq!(status, StatusCode::OK, "{}", body);

    // A void puts the units back into the batches they came from
    let (status, body) = post(&p.router, &format!("/api/v1/sales/{}/void", sale_id), &as_user(&p.owner), json!({ "reason": "Wrong customer" })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(
        batch_on_hand(&p).await,
        vec![("B-OLD".to_string(), qty("9")), ("B-SOON".to_string(), qty("4")), ("B-LATE".to_string(), qty("10"))]
    );
}

#[sqlx::test]
async fn selling_more_than_the_unexpired_batches_hold_is_refused(pool: PgPool) {
    let p = pharmacy(&pool, "fefo-short-owner@example.com").await;
    batch(&p, "B-OLD", -5, "10").await;
    batch(&p, "B-GOOD", 60, "3").await;

    let (status, body) = sell(&p, cart_line(p.drug, p.unit, "5"), 2_500).await;
    assert_eq!(status, StatusCode::CONFLICT, "{}", body);
    assert_eq!(body["code"], "PRODUCT_OUT_OF_STOCK");
    assert_eq!(
        batch_on_hand(&p).await,
        vec![("B-OLD".to_string(), qty("10")), ("B-GOOD".to_string(), qty("3"))]
    );
}

#[sqlx::test]
async fn the_expiry_view_flags_expired_and_near_expiry_batches(pool: PgPool) {
    let p = pharmacy(&pool, "expiry-view-owner@example.com").await;
    batch(&p, "B-OLD", -5, "1").await;
    batch(&p, "B-SOON", 10, "1").await;
    batch(&p, "B-LATE", 300, "1").await;

    let (status, body) = get(&p.router, &format!("/api/v1/stock/expiry?store_id={}", p.store), &as_user(&p.owner)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let rows = body["data"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["batch_number"], "B-OLD");
    assert_eq!(rows[0]["status"], "expired");
    assert_eq!(rows[1]["batch_number"], "B-SOON");
    assert_eq!(rows[1]["status"], "near_expiry");

    let (_, body) = get(&p.router, &format!("/api/v1/stock/expiry?store_id={}&within_days=5", p.store), &as_user(&p.owner)).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
}

#[sqlx::test]
async fn damaged_goods_leave_saleable_stock_and_are_recorded_apart(pool: PgPool) {
    let p = pharmacy(&pool, "write-off-owner@example.com").await;
    let good = batch(&p, "B-GOOD", 60, "10").await;

    let (status, body) = post(
        &p.router,
        "/api/v1/stock/write-offs",
        &as_user(&p.owner),
        json!({ "store_id": p.store, "product_id": p.drug, "stock_item_id": good, "quantity": "2", "condition": "damaged", "reason": "Crushed packs" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(batch_on_hand(&p).await, vec![("B-GOOD".to_string(), qty("8"))]);

    let (status, _) = post(
        &p.router,
        "/api/v1/stock/write-offs",
        &as_user(&p.owner),
        json!({ "store_id": p.store, "product_id": p.drug, "stock_item_id": good, "quantity": "50", "condition": "damaged", "reason": "Too many" }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // A damaged return isn't restocked, but it is recorded
    let (_, sale) = sell(&p, cart_line(p.drug, p.unit, "1"), 500).await;
    let line_id = sale["data"]["lines"][0]["id"].as_str().unwrap();
    let (status, body) = post(
        &p.router,
        &format!("/api/v1/sales/{}/returns", id_of(&sale)),
        &as_user(&p.owner),
        json!({
            "id": Uuid::now_v7(),
            "device_id": "till-1",
            "reason_code": "faulty",
            "lines": [{ "sale_line_id": line_id, "quantity": "1", "condition": "damaged" }],
            "refund": { "method": "original" },
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(batch_on_hand(&p).await, vec![("B-GOOD".to_string(), qty("7"))]);

    let (status, body) = get(&p.router, &format!("/api/v1/stock/unsellable?store_id={}", p.store), &as_user(&p.owner)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let records = body["data"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["source"], "sale_return");
    assert_eq!(records[0]["stock_item_id"], good.to_string());
    assert_eq!(records[1]["source"], "write_off");
    assert_eq!(records[1]["condition"], "damaged");
}

#[sqlx::test]
async fn receiving_goods_creates_or_tops_up_batches(pool: PgPool) {
    let p = pharmacy(&pool, "grn-batch-owner@example.com").await;
    let caller = as_user(&p.owner);
    let (_, supplier) = post(&p.router, "/api/v1/suppliers", &caller, json!({ "name": "MedSupply" })).await;
    let (status, order) = post(
        &p.router,
        "/api/v1/purchase-orders",
        &caller,
        json!({ "store_id": p.store, "supplier_id": id_of(&supplier), "lines": [{ "product_id": p.drug, "quantity_ordered": "30", "unit_cost_amount_minor": 200 }] }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", order);
    let order_id = id_of(&order);
    let (_, lines) = get(&p.router, &format!("/api/v1/purchase-orders/{}/lines", order_id), &caller).await;
    let line_id = lines["data"][0]["id"].as_str().unwrap().to_string();
    let receive = |batch_number: Option<&str>, quantity: &str| {
        json!({ "lines": [{ "purchase_order_line_id": line_id, "quantity_received": quantity, "batch_number": batch_number, "expiry_date": days_from_today(90) }] })
    };
    let path = format!("/api/v1/purchase-orders/{}/receive", order_id);

    let (status, body) = post(&p.router, &path, &caller, receive(None, "10")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{}", body);
    assert_eq!(body["code"], "BATCH_REQUIRED");

    let (status, body) = post(&p.router, &path, &caller, receive(Some("LOT-7"), "10")).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let (status, body) = post(&p.router, &path, &caller, receive(Some("LOT-7"), "5")).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(batch_on_hand(&p).await, vec![("LOT-7".to_string(), qty("15"))]);
}
