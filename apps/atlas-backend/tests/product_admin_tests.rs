//! FR-07 to FR-09 catalogue administration: product search and archiving, categories, variants, and price history.

mod common;

use axum::http::StatusCode;
use axum::Router;
use common::*;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

async fn category(router: &Router, token: &str, name: &str) -> String {
    let (status, body) = post(router, "/api/v1/product-categories", &as_user(token), json!({ "name": name })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    return id_of(&body);
}

async fn names(router: &Router, token: &str, query: &str) -> Vec<String> {
    let (status, body) = get(router, &format!("/api/v1/products{}", query), &as_user(token)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    return body["data"].as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap().to_string()).collect();
}

#[sqlx::test]
async fn products_are_searchable_by_name_sku_barcode_category_and_status(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "search-owner@example.com").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let drinks = category(&router, &owner, "Drinks").await;
    let cola = create_product(&router, &owner, unit, "Cola 50cl", "COLA-50").await;
    let soap = create_product(&router, &owner, unit, "Bar Soap", "SOAP-1").await;
    barcode(&router, &owner, soap, None, "6001234567890").await;
    let (status, _) = patch(&router, &format!("/api/v1/products/{}", cola), &as_user(&owner), json!({ "category_id": drinks })).await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(names(&router, &owner, "?q=cola").await, vec!["Cola 50cl"]);
    assert_eq!(names(&router, &owner, "?q=soap-1").await, vec!["Bar Soap"]);
    assert_eq!(names(&router, &owner, "?q=6001234567890").await, vec!["Bar Soap"]);
    assert_eq!(names(&router, &owner, &format!("?category_id={}", drinks)).await, vec!["Cola 50cl"]);

    // Archived products drop out of the active list but keep their record
    let (status, _) = patch(&router, &format!("/api/v1/products/{}", soap), &as_user(&owner), json!({ "is_active": false })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(names(&router, &owner, "?status=active").await, vec!["Cola 50cl"]);
    assert_eq!(names(&router, &owner, "?status=archived").await, vec!["Bar Soap"]);
    let (status, _) = get(&router, &format!("/api/v1/products/{}", soap), &as_user(&owner)).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn an_archived_category_keeps_its_products_but_takes_no_new_ones(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "category-owner@example.com").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let caller = as_user(&owner);
    let snacks = category(&router, &owner, "Snacks").await;
    let chips = create_product(&router, &owner, unit, "Chips", "CHIPS").await;
    let nuts = create_product(&router, &owner, unit, "Nuts", "NUTS").await;
    patch(&router, &format!("/api/v1/products/{}", chips), &caller, json!({ "category_id": snacks })).await;

    let (status, body) = patch(&router, &format!("/api/v1/product-categories/{}", snacks), &caller, json!({ "name": "Snack Foods" })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["name"], "Snack Foods");

    let (status, body) = post(&router, &format!("/api/v1/product-categories/{}/archive", snacks), &caller, json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert!(body["data"]["archived_at"].is_string());

    let (_, body) = get(&router, "/api/v1/product-categories", &caller).await;
    assert!(body["data"].as_array().unwrap().is_empty());
    let (_, body) = get(&router, "/api/v1/product-categories?include_archived=true", &caller).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);

    assert_eq!(names(&router, &owner, &format!("?category_id={}", snacks)).await, vec!["Chips"]);
    let (status, body) = patch(&router, &format!("/api/v1/products/{}", nuts), &caller, json!({ "category_id": snacks })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{}", body);

    let (status, _) = post(&router, &format!("/api/v1/product-categories/{}/restore", snacks), &caller, json!({})).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = patch(&router, &format!("/api/v1/products/{}", nuts), &caller, json!({ "category_id": snacks })).await;
    assert_eq!(status, StatusCode::OK);
}

async fn create_variant(router: &Router, token: &str, product_type_id: &str, unit: Uuid, parent: Uuid, name: &str, sku: &str) -> (StatusCode, Value) {
    return post(
        router,
        "/api/v1/products",
        &as_user(token),
        json!({ "product_type_id": product_type_id, "base_unit_id": unit, "name": name, "sku": sku, "parent_product_id": parent, "attributes": { "size": name } }),
    )
    .await;
}

#[sqlx::test]
async fn two_variants_are_sold_and_stocked_separately(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "variant-owner@example.com").await;
    let store = create_store(&router, &owner, "variant-store").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let shirt = create_product(&router, &owner, unit, "T-Shirt", "TEE").await;
    let (_, body) = get(&router, &format!("/api/v1/products/{}", shirt), &as_user(&owner)).await;
    let product_type_id = body["data"]["product_type_id"].as_str().unwrap().to_string();

    let (status, body) = create_variant(&router, &owner, &product_type_id, unit, shirt, "T-Shirt M", "TEE-M").await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let medium = Uuid::parse_str(&id_of(&body)).unwrap();
    assert_eq!(body["data"]["parent_product_id"], shirt.to_string());
    let (_, body) = create_variant(&router, &owner, &product_type_id, unit, shirt, "T-Shirt L", "TEE-L").await;
    let large = Uuid::parse_str(&id_of(&body)).unwrap();

    // One level only
    let (status, _) = create_variant(&router, &owner, &product_type_id, unit, medium, "T-Shirt M Slim", "TEE-MS").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let mut variants = names(&router, &owner, &format!("?parent_product_id={}", shirt)).await;
    variants.sort();
    assert_eq!(variants, vec!["T-Shirt L", "T-Shirt M"]);

    for (variant, price) in [(medium, 5_000), (large, 6_000)] {
        list_at(&router, &owner, variant, store).await;
        price_rule(&router, &owner, variant, None, price).await;
        stock_up(&router, &owner, store, variant, "10").await;
    }
    let shift = open_shift(&router, &owner, store).await;
    let cash = method_id(&router, &owner, "cash").await;
    let (status, body) = post(
        &router,
        "/api/v1/sales",
        &as_user(&owner),
        sale_request(store, shift, vec![cart_line(medium, unit, "3")], vec![pay(cash, 15_000)]),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);

    assert_eq!(on_hand(&router, &owner, store, medium).await.parse::<f64>().unwrap(), 7.0);
    assert_eq!(on_hand(&router, &owner, store, large).await.parse::<f64>().unwrap(), 10.0);
}

#[sqlx::test]
async fn price_history_records_previous_and_new_prices(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "price-history-owner@example.com").await;
    let store = create_store(&router, &owner, "price-history").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let rice = sellable(&router, &owner, store, unit, "Rice", "RICE-PH", 1_000, "10").await;
    let caller = as_user(&owner);

    // Sell at the old price first
    let shift = open_shift(&router, &owner, store).await;
    let cash = method_id(&router, &owner, "cash").await;
    let (_, sale) = post(&router, "/api/v1/sales", &caller, sale_request(store, shift, vec![cart_line(rice, unit, "1")], vec![pay(cash, 1_000)])).await;
    let sale_id = id_of(&sale);

    let new_rule = price_rule(&router, &owner, rice, None, 1_200).await;
    let (status, _) = call(
        &router,
        "PUT",
        &format!("/api/v1/products/{}/listings/{}", rice, store),
        Some(&caller),
        Some(json!({ "cost_amount_minor": 800, "low_stock_threshold": null, "is_active": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = delete(&router, &format!("/api/v1/products/{}/price-rules/{}", rice, new_rule), &caller).await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = get(&router, &format!("/api/v1/products/{}/price-history", rice), &caller).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let history = body["data"].as_array().unwrap();
    assert_eq!(history.len(), 4);
    // Newest first: rule removed, cost set, 1,000 -> 1,200, first price
    assert_eq!(history[0]["kind"], "selling");
    assert_eq!(history[0]["previous_amount_minor"], 1_200);
    assert!(history[0]["new_amount_minor"].is_null());
    assert_eq!(history[1]["kind"], "cost");
    assert!(history[1]["previous_amount_minor"].is_null());
    assert_eq!(history[1]["new_amount_minor"], 800);
    assert_eq!(history[2]["previous_amount_minor"], 1_000);
    assert_eq!(history[2]["new_amount_minor"], 1_200);
    assert!(history[2]["changed_by_user_id"].is_string());
    assert!(history[2]["effective_from"].is_string());
    assert!(history[3]["previous_amount_minor"].is_null());

    // The completed sale keeps the price it was sold at
    let (_, body) = get(&router, &format!("/api/v1/sales/{}", sale_id), &caller).await;
    assert_eq!(body["data"]["lines"][0]["unit_price_amount_minor"], 1_000);
}
