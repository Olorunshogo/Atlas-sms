//! Tenant isolation on the tables added for sales, payments, receipts, returns and customers.
//! `#[sqlx::test]` connects as a superuser, which bypasses row level security entirely, so this test switches to the real `atlas_api` role inside a transaction before reading.

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

const TABLES: [&str; 14] = [
    "tax_rates",
    "product_barcodes",
    "stock_exceptions",
    "payment_methods",
    "store_receipt_sequences",
    "sale_receipts",
    "receipt_prints",
    "returns",
    "return_lines",
    "store_return_sequences",
    "customer_credit_movements",
    "refunds",
    "sales",
    "customers",
];

async fn visible_rows(pool: &PgPool, tenant_id: Uuid, table: &str) -> i64 {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SET LOCAL ROLE atlas_api").execute(&mut *tx).await.unwrap();
    sqlx::query("SELECT set_config('app.current_tenant_id', $1, true)")
        .bind(tenant_id.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM atlas.{}", table))
        .fetch_one(&mut *tx)
        .await
        .unwrap_or_else(|err| panic!("atlas_api cannot read atlas.{}: {}", table, err));
    tx.rollback().await.unwrap();
    return count;
}

#[sqlx::test]
async fn another_tenant_sees_none_of_a_tenants_sales_data(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_a) = signup_owner(&router, &pool, "rls-a@example.com").await;
    let (_, tenant_b) = signup_owner(&router, &pool, "rls-b@example.com").await;

    let store = create_store(&router, &owner, "lagos").await;
    let each = seed_unit(&pool, tenant_a, "Each", "ea").await;
    let vat = tax_rate(&router, &owner, "VAT", 750).await;
    store_settings(&router, &owner, store, json!({ "default_tax_rate_id": vat })).await;
    let soap = sellable(&router, &owner, store, each, "Soap", "SOAP", 500, "10").await;
    barcode(&router, &owner, soap, None, "123456").await;
    let shift = open_shift(&router, &owner, store).await;
    let cash = method_id(&router, &owner, "cash").await;
    let (_, customer) = post(&router, "/api/v1/customers", &as_user(&owner), json!({ "full_name": "Tunde" })).await;

    let mut request = sale_request(store, shift, vec![cart_line(soap, each, "2")], vec![pay(cash, 1_000)]);
    request["customer_id"] = customer["data"]["id"].clone();
    let (status, sale) = post(&router, "/api/v1/sync/sales", &as_user(&owner), request).await;
    assert_eq!(status, StatusCode::OK, "{}", sale);
    let sale_id = id_of(&sale);
    let line_id = sale["data"]["lines"][0]["id"].as_str().unwrap();
    let (status, _) = post(&router, &format!("/api/v1/sales/{}/receipt/prints", sale_id), &as_user(&owner), json!({})).await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = post(
        &router,
        &format!("/api/v1/sales/{}/returns", sale_id),
        &as_user(&owner),
        json!({
            "id": Uuid::now_v7(),
            "reason_code": "faulty",
            "lines": [{ "sale_line_id": line_id, "quantity": "1", "condition": "resellable" }],
            "refund": { "method": "store_credit" },
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    // An oversold synced sale leaves a stock exception behind.
    let request = sale_request(store, shift, vec![cart_line(soap, each, "50")], vec![pay(cash, 25_000)]);
    let (status, _) = post(&router, "/api/v1/sync/sales", &as_user(&owner), request).await;
    assert_eq!(status, StatusCode::OK);

    for table in TABLES {
        assert!(visible_rows(&pool, tenant_a, table).await > 0, "tenant A should see its own atlas.{}", table);
        let leaked = visible_rows(&pool, tenant_b, table).await;
        let own = if table == "payment_methods" { 4 } else { 0 };
        assert_eq!(leaked, own, "tenant B sees tenant A's atlas.{}", table);
    }
}
