//! FR-15 cart building: product search, barcode scanning, server-side price and tax resolution, and the quote a till reviews before charging.

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{json, Value};
use sqlx::PgPool;

fn line(product_id: uuid::Uuid, unit_id: uuid::Uuid, quantity: &str) -> Value {
    return json!({ "product_id": product_id, "unit_id": unit_id, "quantity": quantity });
}

#[sqlx::test]
async fn search_and_barcode_scan_return_store_prices_units_and_stock(pool: PgPool) {
    let router = build_router(pool.clone());
    let (token, tenant_id) = signup_owner(&router, &pool, "catalog-owner@example.com").await;
    let lagos = create_store(&router, &token, "lagos").await;
    let kg = seed_unit(&pool, tenant_id, "Kilogram", "kg").await;
    let bag = seed_unit(&pool, tenant_id, "Bag 50kg", "bag").await;

    let rice = sellable(&router, &token, lagos, kg, "Long Grain Rice", "RICE", 1_500, "1000").await;
    sale_unit(&router, &token, rice, bag, "50").await;
    barcode(&router, &token, rice, Some(bag), "6151234500017").await;
    // Listed nowhere - must not appear in Lagos's catalog.
    create_product(&router, &token, kg, "Rice Flour", "FLOUR").await;

    let caller = as_user(&token);
    let (status, body) = get(&router, &format!("/api/v1/stores/{}/catalog?q=rice", lagos), &caller).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let items = body["data"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "Long Grain Rice");
    let units = items[0]["units"].as_array().unwrap();
    let bag_unit = units.iter().find(|u| u["unit_id"] == bag.to_string()).unwrap();
    assert_eq!(bag_unit["unit_price_amount_minor"], 75_000);
    assert_eq!(bag_unit["barcodes"][0], "6151234500017");

    let (status, body) = get(&router, &format!("/api/v1/stores/{}/catalog/barcode/6151234500017", lagos), &caller).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["unit_id"], bag.to_string());
    assert_eq!(body["data"]["item"]["product_id"], rice.to_string());

    let (status, _) = get(&router, &format!("/api/v1/stores/{}/catalog/barcode/0000", lagos), &caller).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn quote_prices_tax_inclusive_and_exempt_lines_on_the_server(pool: PgPool) {
    let router = build_router(pool.clone());
    let (token, tenant_id) = signup_owner(&router, &pool, "quote-owner@example.com").await;
    let store = create_store(&router, &token, "lagos").await;
    let each = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let vat = tax_rate(&router, &token, "VAT", 750).await;
    let exempt = tax_rate(&router, &token, "Exempt", 0).await;
    store_settings(&router, &token, store, json!({ "default_tax_rate_id": vat, "prices_include_tax": true })).await;

    let radio = sellable(&router, &token, store, each, "Radio", "RADIO", 107_500, "10").await;
    let bread = sellable(&router, &token, store, each, "Bread", "BREAD", 1_000, "10").await;
    let (status, _) = patch(&router, &format!("/api/v1/products/{}", bread), &as_user(&token), json!({ "tax_rate_id": exempt })).await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = post(
        &router,
        &format!("/api/v1/stores/{}/catalog/quote", store),
        &as_user(&token),
        json!({ "customer_id": null, "lines": [line(radio, each, "1"), line(bread, each, "3")], "cart_discount": null }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let quote = &body["data"];
    assert_eq!(quote["total_amount_minor"], 110_500);
    assert_eq!(quote["tax_amount_minor"], 7_500);
    assert_eq!(quote["lines"][0]["tax_rate_basis_points"], 750);
    assert_eq!(quote["lines"][1]["tax_amount_minor"], 0);
    assert_eq!(quote["violations"].as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn quote_flags_overrides_discounts_and_short_stock_for_a_cashier(pool: PgPool) {
    let router = build_router(pool.clone());
    let (token, tenant_id) = signup_owner(&router, &pool, "violations-owner@example.com").await;
    let store = create_store(&router, &token, "lagos").await;
    let each = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let soap = sellable(&router, &token, store, each, "Soap", "SOAP", 500, "2").await;
    let (_, cashier) = seed_staff(&pool, tenant_id, store, "cashier").await;

    let cart = json!({
        "customer_id": null,
        "lines": [{
            "product_id": soap, "unit_id": each, "quantity": "3",
            "unit_price_amount_minor": 400, "price_override_reason": "damaged box"
        }],
        "cart_discount": { "amount_minor": 100, "reason": "loyal customer" },
    });
    let path = format!("/api/v1/stores/{}/catalog/quote", store);

    let (status, body) = post(&router, &path, &as_user(&cashier), cart.clone()).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let codes: Vec<&str> = body["data"]["violations"].as_array().unwrap().iter().map(|v| v["code"].as_str().unwrap()).collect();
    assert!(codes.contains(&"PRICE_OVERRIDE_FORBIDDEN"), "{:?}", codes);
    assert!(codes.contains(&"DISCOUNT_FORBIDDEN"), "{:?}", codes);
    assert!(codes.contains(&"PRODUCT_OUT_OF_STOCK"), "{:?}", codes);

    // The owner holds sale.price_override and sale.discount; only the stock problem remains.
    let (status, body) = post(&router, &path, &as_user(&token), cart).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let codes: Vec<&str> = body["data"]["violations"].as_array().unwrap().iter().map(|v| v["code"].as_str().unwrap()).collect();
    assert_eq!(codes, vec!["PRODUCT_OUT_OF_STOCK"]);
    assert_eq!(body["data"]["total_amount_minor"], 1_100);
    assert_eq!(body["data"]["lines"][0]["list_unit_price_amount_minor"], 500);
}

#[sqlx::test]
async fn the_most_specific_price_rule_wins(pool: PgPool) {
    let router = build_router(pool.clone());
    let (token, tenant_id) = signup_owner(&router, &pool, "rules-owner@example.com").await;
    let lagos = create_store(&router, &token, "lagos").await;
    let abuja = create_store(&router, &token, "abuja").await;
    let each = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let oil = sellable(&router, &token, lagos, each, "Oil", "OIL", 2_000, "100").await;
    list_at(&router, &token, oil, abuja).await;
    price_rule(&router, &token, oil, Some(lagos), 1_800).await;
    // Wholesale break: 10 or more at Lagos.
    let (status, _) = post(
        &router,
        &format!("/api/v1/products/{}/price-rules", oil),
        &as_user(&token),
        json!({ "store_id": lagos, "unit_price_amount_minor": 1_500, "min_quantity": "10" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let quote = |store: uuid::Uuid, qty: &'static str| {
        let router = router.clone();
        let token = token.clone();
        async move {
            let (status, body) = post(
                &router,
                &format!("/api/v1/stores/{}/catalog/quote", store),
                &as_user(&token),
                json!({ "customer_id": null, "lines": [line(oil, each, qty)], "cart_discount": null }),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{}", body);
            body["data"]["lines"][0]["unit_price_amount_minor"].as_i64().unwrap()
        }
    };
    assert_eq!(quote(abuja, "1").await, 2_000);
    assert_eq!(quote(lagos, "1").await, 1_800);
    assert_eq!(quote(lagos, "10").await, 1_500);
}

#[sqlx::test]
async fn an_unlisted_product_cannot_be_quoted(pool: PgPool) {
    let router = build_router(pool.clone());
    let (token, tenant_id) = signup_owner(&router, &pool, "unlisted-owner@example.com").await;
    let store = create_store(&router, &token, "lagos").await;
    let each = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let product = create_product(&router, &token, each, "Ghost", "GHOST").await;
    price_rule(&router, &token, product, None, 100).await;

    let (status, body) = post(
        &router,
        &format!("/api/v1/stores/{}/catalog/quote", store),
        &as_user(&token),
        json!({ "customer_id": null, "lines": [line(product, each, "1")], "cart_discount": null }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PRODUCT_NOT_SOLD_HERE");
}
